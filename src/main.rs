#![cfg_attr(windows, windows_subsystem = "windows")]

mod bootstrap;
mod checker;
mod dsh_process;
mod splash;

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};

use bootstrap::{run_bootstrap, BootstrapControl, BootstrapState, UiMsg, DSH_URL};
use dsh_process::DshProcess;
use splash::{apply_msg, build_splash_html};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{
    ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget,
};
use tao::window::{Window, WindowBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use wry::{WebContext, WebViewBuilder};

enum UserEvent {
    Bootstrap(u64, UiMsg),
    BootstrapComplete(u64),
    Tray(TrayIconEvent),
    Menu(MenuEvent),
    Retry,
    Exit,
}

#[derive(Debug, PartialEq, Eq)]
enum DesktopAction {
    Hide,
    Show,
    Exit,
}

struct DesktopState {
    window: Window,
    webview: wry::WebView,
    _web_context: WebContext,
    _tray: TrayIcon,
}

#[derive(Debug, PartialEq)]
struct WindowGeometry {
    width: f64,
    height: f64,
    minimum_width: f64,
    minimum_height: f64,
    resizable: bool,
}

fn main() -> wry::Result<()> {
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let event_proxy = proxy.clone();
    let tray_event_proxy = proxy.clone();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = tray_event_proxy.send_event(UserEvent::Tray(event));
    }));
    let menu_event_proxy = proxy.clone();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_event_proxy.send_event(UserEvent::Menu(event));
    }));
    let desktop = open_desktop(&event_loop, proxy.clone()).map_err(std::io::Error::other)?;
    let managed_process = Arc::new(Mutex::new(None::<DshProcess>));
    let bootstrap_state = Arc::new(BootstrapState::default());
    let bootstrap_control = BootstrapControl::new();
    let initial_generation = bootstrap_state
        .start()
        .expect("initial bootstrap should start");
    launch_bootstrap(proxy, bootstrap_control.clone(), initial_generation);

    event_loop.run(move |event, _event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::Bootstrap(generation, message)) => {
                if !bootstrap_state.is_current(generation) {
                    if let UiMsg::Done(process) = message {
                        process.stop();
                    }
                    return;
                }

                match message {
                    UiMsg::Done(process) => {
                        if let Err(error) =
                            desktop.webview.evaluate_script(&ready_navigation_script())
                        {
                            process.stop();
                            if let Some(process) = managed_process
                                .lock()
                                .expect("managed process lock poisoned")
                                .take()
                            {
                                process.stop();
                            }
                            let _ = apply_msg(
                                &desktop.webview,
                                &UiMsg::Fail(format!("加载主界面失败: {error}")),
                            );
                        } else {
                            *managed_process
                                .lock()
                                .expect("managed process lock poisoned") = Some(process);
                        }
                    }
                    message => {
                        let _ = apply_msg(&desktop.webview, &message);
                    }
                }
            }
            Event::UserEvent(UserEvent::BootstrapComplete(generation)) => {
                bootstrap_state.finish(generation);
            }
            Event::UserEvent(UserEvent::Tray(TrayIconEvent::Click {
                button,
                button_state,
                ..
            })) => match tray_click_action(button, button_state) {
                Some(DesktopAction::Show) => show_main(Some(&desktop)),
                Some(DesktopAction::Hide | DesktopAction::Exit) | None => {}
            },
            Event::UserEvent(UserEvent::Menu(event)) => match menu_action(event.id.as_ref()) {
                Some(DesktopAction::Show) => show_main(Some(&desktop)),
                Some(DesktopAction::Exit) => {
                    exit_application(&bootstrap_control, &managed_process, control_flow)
                }
                Some(DesktopAction::Hide) | None => {}
            },
            Event::UserEvent(UserEvent::Retry) => {
                if let Some(generation) = bootstrap_state.start() {
                    let _ = desktop.webview.evaluate_script("reset();");
                    launch_bootstrap(event_proxy.clone(), bootstrap_control.clone(), generation);
                }
            }
            Event::UserEvent(UserEvent::Exit) => {
                exit_application(&bootstrap_control, &managed_process, control_flow)
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => match window_close_action(desktop.window.id() == window_id) {
                Some(DesktopAction::Hide) => desktop.window.set_visible(false),
                Some(DesktopAction::Show | DesktopAction::Exit) | None => {}
            },
            _ => {}
        }
    });
}

fn open_desktop(
    event_loop: &EventLoopWindowTarget<UserEvent>,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<DesktopState, String> {
    let icon = window_icon()?;
    let geometry = initial_window_geometry();
    let window = WindowBuilder::new()
        .with_title("DeepSeek Harness Desktop")
        .with_inner_size(tao::dpi::LogicalSize::new(geometry.width, geometry.height))
        .with_min_inner_size(tao::dpi::LogicalSize::new(
            geometry.minimum_width,
            geometry.minimum_height,
        ))
        .with_resizable(geometry.resizable)
        .with_window_icon(Some(icon))
        .with_background_color((0, 0, 0, 255))
        .build(event_loop)
        .map_err(|error| format!("创建主窗口失败: {error}"))?;
    center_window(&window, event_loop);
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "未设置 LOCALAPPDATA 环境变量".to_string())?;
    let mut web_context = WebContext::new(Some(webview_data_directory(Path::new(&local_app_data))));
    let webview = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_html(build_splash_html())
        .with_ipc_handler(move |request| match request.body().as_str() {
            "retry" => {
                let _ = proxy.send_event(UserEvent::Retry);
            }
            "exit" => {
                let _ = proxy.send_event(UserEvent::Exit);
            }
            _ => {}
        })
        .build(&window)
        .map_err(|error| format!("创建主 WebView 失败: {error}"))?;

    let icon = tray_icon()?;
    let menu = Menu::new();
    let show = MenuItem::with_id("show", "显示主窗口", true, None);
    let quit = MenuItem::with_id("quit", "退出", true, None);
    menu.append(&show)
        .map_err(|error| format!("创建托盘菜单失败: {error}"))?;
    menu.append(&quit)
        .map_err(|error| format!("创建托盘菜单失败: {error}"))?;
    let tray = TrayIconBuilder::new()
        .with_tooltip("DeepSeek Harness Desktop")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()
        .map_err(|error| format!("创建托盘失败: {error}"))?;

    Ok(DesktopState {
        window,
        webview,
        _web_context: web_context,
        _tray: tray,
    })
}

fn ready_navigation_script() -> String {
    format!("window.location.replace({DSH_URL:?});")
}

fn ready_window_geometry() -> WindowGeometry {
    WindowGeometry {
        width: 1280.0,
        height: 800.0,
        minimum_width: 800.0,
        minimum_height: 600.0,
        resizable: true,
    }
}

fn initial_window_geometry() -> WindowGeometry {
    ready_window_geometry()
}

fn webview_data_directory(local_app_data: &Path) -> PathBuf {
    local_app_data.join("DSH Desktop").join("WebView2")
}

fn center_window(window: &Window, event_loop: &EventLoopWindowTarget<UserEvent>) {
    let Some(monitor) = event_loop.primary_monitor() else {
        return;
    };
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let window_size = window.outer_size();
    let x = monitor_position.x + (monitor_size.width as i32 - window_size.width as i32) / 2;
    let y = monitor_position.y + (monitor_size.height as i32 - window_size.height as i32) / 2;
    window.set_outer_position(tao::dpi::PhysicalPosition::new(x, y));
}

fn launch_bootstrap(proxy: EventLoopProxy<UserEvent>, control: BootstrapControl, generation: u64) {
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let worker_control = control.clone();
        let bootstrap_thread = std::thread::spawn(move || run_bootstrap(tx, &worker_control));
        for message in rx {
            let process_to_stop = match &message {
                UiMsg::Done(process) => Some(process.clone()),
                _ => None,
            };
            if proxy
                .send_event(UserEvent::Bootstrap(generation, message))
                .is_err()
            {
                if let Some(process) = process_to_stop {
                    process.stop();
                }
                break;
            }
        }
        let _ = bootstrap_thread.join();
        let _ = proxy.send_event(UserEvent::BootstrapComplete(generation));
    });
}

fn tray_icon() -> Result<Icon, String> {
    let (rgba, width, height) = icon_rgba()?;
    Icon::from_rgba(rgba, width, height).map_err(|error| format!("创建托盘图标失败: {error}"))
}

fn window_icon() -> Result<tao::window::Icon, String> {
    let (rgba, width, height) = icon_rgba()?;
    tao::window::Icon::from_rgba(rgba, width, height)
        .map_err(|error| format!("创建主窗口图标失败: {error}"))
}

fn icon_rgba() -> Result<(Vec<u8>, u32, u32), String> {
    let image = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .map_err(|error| format!("图标解码失败: {error}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Ok((image.into_raw(), width, height))
}

fn show_main(desktop: Option<&DesktopState>) {
    if let Some(desktop) = desktop {
        desktop.window.set_visible(true);
        desktop.window.set_focus();
    }
}

fn window_close_action(is_desktop_window: bool) -> Option<DesktopAction> {
    is_desktop_window.then_some(DesktopAction::Hide)
}

fn tray_click_action(button: MouseButton, state: MouseButtonState) -> Option<DesktopAction> {
    (button == MouseButton::Left && state == MouseButtonState::Up).then_some(DesktopAction::Show)
}

fn menu_action(id: &str) -> Option<DesktopAction> {
    match id {
        "show" => Some(DesktopAction::Show),
        "quit" => Some(DesktopAction::Exit),
        _ => None,
    }
}

fn exit_application(
    bootstrap_control: &BootstrapControl,
    managed_process: &Arc<Mutex<Option<DshProcess>>>,
    control_flow: &mut ControlFlow,
) {
    bootstrap_control.cancel();
    if let Some(process) = managed_process
        .lock()
        .expect("managed process lock poisoned")
        .as_ref()
    {
        process.stop();
    }
    *control_flow = ControlFlow::Exit;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_action_recognizes_show_and_quit() {
        assert_eq!(menu_action("show"), Some(DesktopAction::Show));
        assert_eq!(menu_action("quit"), Some(DesktopAction::Exit));
        assert_eq!(menu_action("unknown"), None);
    }

    #[test]
    fn ready_navigation_targets_the_dsh_url() {
        assert_eq!(
            ready_navigation_script(),
            "window.location.replace(\"http://127.0.0.1:3080\");"
        );
    }

    #[test]
    fn ready_navigation_replaces_the_loading_page() {
        let script = ready_navigation_script();

        assert!(script.contains("window.location.replace"));
        assert!(script.contains(DSH_URL));
    }

    #[test]
    fn ready_window_geometry_restores_the_main_window() {
        assert_eq!(
            ready_window_geometry(),
            WindowGeometry {
                width: 1280.0,
                height: 800.0,
                minimum_width: 800.0,
                minimum_height: 600.0,
                resizable: true,
            }
        );
    }

    #[test]
    fn initial_window_geometry_matches_the_ready_window() {
        assert_eq!(initial_window_geometry(), ready_window_geometry());
    }

    #[test]
    fn webview_data_directory_uses_local_app_data() {
        assert_eq!(
            webview_data_directory(std::path::Path::new(r"C:\Users\test\AppData\Local")),
            std::path::PathBuf::from(r"C:\Users\test\AppData\Local\DSH Desktop\WebView2")
        );
    }

    #[test]
    fn tray_click_opens_only_on_left_button_release() {
        assert_eq!(
            tray_click_action(MouseButton::Left, MouseButtonState::Up),
            Some(DesktopAction::Show)
        );
        assert_eq!(
            tray_click_action(MouseButton::Left, MouseButtonState::Down),
            None
        );
        assert_eq!(
            tray_click_action(MouseButton::Right, MouseButtonState::Up),
            None
        );
    }

    #[test]
    fn tray_show_and_close_window_use_the_same_desktop_state() {
        assert_eq!(window_close_action(false), None);
        assert_eq!(window_close_action(true), Some(DesktopAction::Hide));
        assert_eq!(
            tray_click_action(MouseButton::Left, MouseButtonState::Up),
            Some(DesktopAction::Show)
        );
        assert_eq!(menu_action("show"), Some(DesktopAction::Show));
        assert_eq!(menu_action("quit"), Some(DesktopAction::Exit));
        assert_eq!(menu_action("unknown"), None);
        assert_eq!(
            tray_click_action(MouseButton::Right, MouseButtonState::Up),
            None
        );
        assert_eq!(
            tray_click_action(MouseButton::Left, MouseButtonState::Down),
            None
        );
    }

    #[test]
    fn icon_png_decodes_to_rgba() {
        let image =
            image::load_from_memory(include_bytes!("../assets/icon.png")).expect("PNG 解码失败");
        assert_eq!(image.width(), 256);
        assert!(
            image.to_rgba8().pixels().any(|pixel| pixel[3] > 0),
            "应有非透明像素"
        );
    }

    #[test]
    fn official_icon_creates_window_icon() {
        window_icon().expect("主窗口图标应可从内嵌 PNG 创建");
    }
}
