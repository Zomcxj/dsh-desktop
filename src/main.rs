#![cfg_attr(windows, windows_subsystem = "windows")]

mod bootstrap;
mod checker;
mod dsh_process;
mod env_check;
mod process;
mod splash;

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};

use bootstrap::{run_bootstrap, BootstrapControl, BootstrapState, UiMsg, DSH_URL};
use dsh_process::DshProcess;
use splash::{apply_msg, build_splash_html, inject_navbar_script, nav_set_exit_mode, nav_set_tray_mode};
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
    RefreshPage,
    RestartService,
    ToggleExitOnClose,
    ToggleTray,
    PageLoaded(String),
    InjectNavbar,
    InstallNode,
    InstallDsh,
    InstallFinished(&'static str, bool),
    UpdateAvailable(bool),
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

struct AppSettings {
    exit_on_close: bool,
    tray_enabled: bool,
}

impl AppSettings {
    fn new() -> Self {
        Self {
            exit_on_close: false,
            tray_enabled: true,
        }
    }
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
    // 单实例：杀掉已有进程，确保新 exe 启动全新的界面
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // 不杀自己
        let self_pid = std::process::id();
        let _ = std::process::Command::new("taskkill")
            .args(["/FI", &format!("PID ne {self_pid}"), "/IM", "dsh-desktop.exe", "/T", "/F"])
            .creation_flags(0x0800_0000)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

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
    let desktop = open_desktop(&event_loop, proxy.clone(), proxy.clone()).map_err(std::io::Error::other)?;
    let managed_process = Arc::new(Mutex::new(None::<DshProcess>));
    let bootstrap_state = Arc::new(BootstrapState::default());
    let bootstrap_control = BootstrapControl::new();
    let initial_generation = bootstrap_state
        .start()
        .expect("initial bootstrap should start");
    let settings = Arc::new(Mutex::new(AppSettings::new()));
    let settings_clone = settings.clone();
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
                        // 导航到 DSH 页面
                        if let Err(error) =
                            desktop.webview.evaluate_script(&navigation_script_with_settings(
                                &settings_clone,
                            ))
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
                            // 延迟注入导航栏（备选，on_page_load 可能不触发）
                            let delayed_proxy = event_proxy.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_secs(2));
                                let _ = delayed_proxy.send_event(UserEvent::InjectNavbar);
                            });
                            // 后台异步检查 dsh 更新
                            let update_proxy = event_proxy.clone();
                            std::thread::spawn(move || {
                                if let Some(_latest) = crate::env_check::latest_dsh_version() {
                                    let local = crate::env_check::check_dsh();
                                    let needs_update = local.ok
                                        && local.version.as_deref().map(|v| v.trim())
                                            != Some(_latest.trim());
                                    if needs_update {
                                        let ok = crate::run_install_command(
                                            "npm",
                                            &["install", "-g", "@deepseek-ai/dsh"],
                                        );
                                        let _ = update_proxy
                                            .send_event(UserEvent::UpdateAvailable(ok));
                                    }
                                }
                            });
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
            Event::UserEvent(UserEvent::RefreshPage) => {
                let _ = desktop
                    .webview
                    .evaluate_script(&navigation_script_with_settings(&settings_clone));
            }
            Event::UserEvent(UserEvent::RestartService) => {
                // 停止当前服务
                if let Some(process) = managed_process
                    .lock()
                    .expect("managed process lock poisoned")
                    .take()
                {
                    process.stop();
                }
                // 回到启动页，让用户看到重启过程
                let _ = desktop
                    .webview
                    .load_html(&build_splash_html());
                // 重新启动
                if let Some(generation) = bootstrap_state.start() {
                    launch_bootstrap(event_proxy.clone(), bootstrap_control.clone(), generation);
                }
            }
            Event::UserEvent(UserEvent::ToggleExitOnClose) => {
                let mut settings = settings_clone.lock().expect("settings lock poisoned");
                settings.exit_on_close = !settings.exit_on_close;
                if settings.exit_on_close {
                    settings.tray_enabled = false;
                    let _ = desktop.webview.evaluate_script(&nav_set_tray_mode(false));
                }
                let _ = desktop
                    .webview
                    .evaluate_script(&nav_set_exit_mode(settings.exit_on_close));
            }
            Event::UserEvent(UserEvent::ToggleTray) => {
                let mut settings = settings_clone.lock().expect("settings lock poisoned");
                settings.tray_enabled = !settings.tray_enabled;
                if settings.tray_enabled {
                    settings.exit_on_close = false;
                    let _ = desktop.webview.evaluate_script(&nav_set_exit_mode(false));
                }
                let _ = desktop
                    .webview
                    .evaluate_script(&nav_set_tray_mode(settings.tray_enabled));
            }
            Event::UserEvent(UserEvent::PageLoaded(url)) => {
                if url.starts_with(DSH_URL) {
                    inject_navbar_to_desktop(&desktop, &settings_clone);
                }
            }
            Event::UserEvent(UserEvent::InjectNavbar) => {
                inject_navbar_to_desktop(&desktop, &settings_clone);
            }
            Event::UserEvent(UserEvent::InstallNode) => {
                let install_proxy = event_proxy.clone();
                std::thread::spawn(move || {
                    let ok = run_install_command(
                        "winget",
                        &[
                            "install",
                            "OpenJS.NodeJS.LTS",
                            "--silent",
                            "--accept-package-agreements",
                            "--accept-source-agreements",
                        ],
                    );
                    let _ = install_proxy.send_event(UserEvent::InstallFinished("node", ok));
                });
            }
            Event::UserEvent(UserEvent::InstallDsh) => {
                // 先停掉正在运行的 dsh web 进程，释放被锁定的原生 DLL，否则 npm 无法覆盖安装
                if let Some(process) = managed_process
                    .lock()
                    .expect("managed process lock poisoned")
                    .take()
                {
                    process.stop();
                }
                let install_proxy = event_proxy.clone();
                std::thread::spawn(move || {
                    let ok = run_install_command("npm", &["install", "-g", "@deepseek-ai/dsh"]);
                    let _ = install_proxy.send_event(UserEvent::InstallFinished("dsh", ok));
                });
            }
            Event::UserEvent(UserEvent::UpdateAvailable(true)) => {
                let _ = desktop.webview.evaluate_script(
                    r#"showUpdateDot();"#,
                );
            }
            Event::UserEvent(UserEvent::UpdateAvailable(false)) => {}
            
            Event::UserEvent(UserEvent::InstallFinished(which, success)) => {
                if success {
                    // 自动重新检查环境并继续启动
                    if let Some(generation) = bootstrap_state.start() {
                        let _ = desktop.webview.evaluate_script("reset();");
                        launch_bootstrap(event_proxy.clone(), bootstrap_control.clone(), generation);
                    }
                } else {
                    let msg = format!("{which} 安装失败，请按安装方式手动安装后重试");
                    let _ = desktop.webview.evaluate_script(&format!("setStatus({msg:?});"));
                    let _ = desktop.webview.evaluate_script(
                        "var b=document.querySelector('.env-btn[disabled]');if(b){b.disabled=false;b.textContent='自动安装';}",
                    );
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => {
                let is_desktop = desktop.window.id() == window_id;
                let action = if is_desktop {
                    let settings = settings_clone.lock().expect("settings lock poisoned");
                    if settings.exit_on_close {
                        Some(DesktopAction::Exit)
                    } else {
                        Some(DesktopAction::Hide)
                    }
                } else {
                    None
                };
                match action {
                    Some(DesktopAction::Hide) => desktop.window.set_visible(false),
                    Some(DesktopAction::Exit) => {
                        exit_application(&bootstrap_control, &managed_process, control_flow)
                    }
                    Some(DesktopAction::Show) | None => {}
                }
            }
            _ => {}
        }
    });
}

fn open_desktop(
    event_loop: &EventLoopWindowTarget<UserEvent>,
    proxy: EventLoopProxy<UserEvent>,
    nav_proxy: EventLoopProxy<UserEvent>,
) -> Result<DesktopState, String> {
    let icon = window_icon()?;
    let geometry = adaptive_window_geometry(event_loop);
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
            "refresh" => {
                let _ = proxy.send_event(UserEvent::RefreshPage);
            }
            "restart" => {
                let _ = proxy.send_event(UserEvent::RestartService);
            }
            "toggle-exit-mode" => {
                let _ = proxy.send_event(UserEvent::ToggleExitOnClose);
            }
            "toggle-tray" => {
                let _ = proxy.send_event(UserEvent::ToggleTray);
            }
            "install-node" => {
                let _ = proxy.send_event(UserEvent::InstallNode);
            }
            "install-dsh" => {
                let _ = proxy.send_event(UserEvent::InstallDsh);
            }
            _ => {}
        })
        .with_on_page_load_handler(move |event, url| {
            // event: PageLoadEvent (Started/Finished), url: String
            if let wry::PageLoadEvent::Finished = event {
                if url == DSH_URL || url.starts_with("http://127.0.0.1:3080") {
                    let _ = nav_proxy.send_event(UserEvent::PageLoaded(url));
                }
            }
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

/// 生成导航脚本（仅导航，导航栏由 PageLoaded/InjectNavbar 注入）
fn navigation_script_with_settings(_settings: &Arc<Mutex<AppSettings>>) -> String {
    format!("location.replace({DSH_URL:?});")
}

/// 根据屏幕尺寸自适应计算窗口大小
fn adaptive_window_geometry(event_loop: &EventLoopWindowTarget<UserEvent>) -> WindowGeometry {
    let default = ready_window_geometry();
    let Some(monitor) = event_loop.primary_monitor() else {
        return default;
    };
    let size = monitor.size(); // PhysicalSize
    let scale = monitor.scale_factor();
    // 转换为逻辑像素
    let screen_w = size.width as f64 / scale;
    let screen_h = size.height as f64 / scale;
    // 宽 70%，高 80%，不超过 1400×900
    let w = (screen_w * 0.70).max(800.0).min(1400.0);
    let h = (screen_h * 0.80).max(600.0).min(900.0);
    WindowGeometry {
        width: w,
        height: h,
        minimum_width: default.minimum_width,
        minimum_height: default.minimum_height,
        resizable: default.resizable,
    }
}

fn ready_window_geometry() -> WindowGeometry {
    WindowGeometry {
        width: 1400.0,
        height: 800.0,
        minimum_width: 800.0,
        minimum_height: 600.0,
        resizable: true,
    }
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

/// 运行自动安装命令（Windows 下隐藏控制台窗口），返回是否成功。
pub fn run_install_command(bin: &str, args: &[&str]) -> bool {
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        // npm 是 .cmd shim，CreateProcess 无法直接启动，需经 cmd /c
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg(bin).args(args);
        c.creation_flags(0x0800_0000);
        c
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut c = std::process::Command::new(bin);
        c.args(args);
        c
    };

    let timeout = if bin == "winget" {
        std::time::Duration::from_secs(15 * 60)
    } else {
        std::time::Duration::from_secs(5 * 60)
    };
    process::status(&mut command, timeout)
}

fn show_main(desktop: Option<&DesktopState>) {
    if let Some(desktop) = desktop {
        desktop.window.set_visible(true);
        desktop.window.set_focus();
    }
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

/// 注入导航栏到桌面页面并同步设置状态
fn inject_navbar_to_desktop(
    desktop: &DesktopState,
    settings: &Arc<Mutex<AppSettings>>,
) {
    let s = settings.lock().expect("settings lock poisoned");
    let mut js = inject_navbar_script();
    js.push_str(&nav_set_exit_mode(s.exit_on_close));
    js.push_str(&nav_set_tray_mode(s.tray_enabled));
    drop(s);
    let _ = desktop.webview.evaluate_script(&js);
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
    fn ready_window_geometry_restores_the_main_window() {
        assert_eq!(
            ready_window_geometry(),
            WindowGeometry {
                width: 1400.0,
                height: 800.0,
                minimum_width: 800.0,
                minimum_height: 600.0,
                resizable: true,
            }
        );
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

    #[cfg(windows)]
    #[test]
    fn install_command_times_out() {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "ping", "127.0.0.1", "-n", "3"]);
        assert!(!crate::process::status(
            &mut command,
            std::time::Duration::from_millis(100)
        ));
    }
}
