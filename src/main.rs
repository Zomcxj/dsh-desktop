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
use splash::{
    apply_msg, build_splash_html, inject_navbar_script, nav_set_exit_mode, nav_set_tray_mode,
};
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
    InstallDshUpdate,
    DshUpdateFinished(Result<String, String>),
    ShowUpdateFailure(String),
    DismissDshUpdate,
    AuthPageProbe(u64, String),
    AuthRetry(u64),
    HideStartupOverlay,
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
    splash_webview: wry::WebView,
    _web_context: WebContext,
    _tray: TrayIcon,
}

struct AppSettings {
    exit_on_close: bool,
    tray_enabled: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum UpdateAction {
    Restart,
    FailInstall,
    FailValidation,
}

struct AuthRetryState {
    session: u64,
    url: String,
    attempts: u8,
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
            .args([
                "/FI",
                &format!("PID ne {self_pid}"),
                "/IM",
                "dsh-desktop.exe",
                "/T",
                "/F",
            ])
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
    let desktop =
        open_desktop(&event_loop, proxy.clone(), proxy.clone()).map_err(std::io::Error::other)?;
    let managed_process = Arc::new(Mutex::new(None::<DshProcess>));
    let bootstrap_state = Arc::new(BootstrapState::default());
    let bootstrap_control = BootstrapControl::new();
    let initial_generation = bootstrap_state
        .start()
        .expect("initial bootstrap should start");
    let settings = Arc::new(Mutex::new(AppSettings::new()));
    let settings_clone = settings.clone();
    launch_bootstrap(proxy, bootstrap_control.clone(), initial_generation);
    let mut auth_retry = None::<AuthRetryState>;
    let mut auth_session = 0_u64;
    let mut dsh_update_in_progress = false;

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
                        let target = process
                            .authenticated_url()
                            .unwrap_or_else(|| DSH_URL.to_string());
                        auth_session += 1;
                        auth_retry = target.contains("token=").then(|| AuthRetryState {
                            session: auth_session,
                            url: target.clone(),
                            attempts: 0,
                        });
                        // 直接打开 token URL，让 WebView 接收 dsh 返回的 HttpOnly cookie。
                        if let Err(error) =
                            desktop.webview.evaluate_script(&navigation_script(&target))
                        {
                            process.stop();
                            if let Some(process) = managed_process
                                .lock()
                                .expect("managed process lock poisoned")
                                .take()
                            {
                                process.stop();
                            }
                            auth_retry = None;
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
                            // 后台只检查更新，不在 dsh 运行时修改全局 node_modules。
                            let update_proxy = event_proxy.clone();
                            std::thread::spawn(move || {
                                if let Some(_latest) = crate::env_check::latest_dsh_version() {
                                    let local = crate::env_check::check_dsh();
                                    let needs_update = local.ok
                                        && local.version.as_deref().map(|v| v.trim())
                                            != Some(_latest.trim());
                                    if needs_update {
                                        let _ = update_proxy.send_event(UserEvent::UpdateAvailable(true));
                                    }
                                }
                            });
                        }
                    }
                    message => {
                        let _ = apply_msg(&desktop.splash_webview, &message);
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
                    auth_retry = None;
                    let _ = desktop.splash_webview.evaluate_script("reset();");
                    let _ = desktop.webview.set_visible(false);
                    let _ = desktop.splash_webview.set_visible(true);
                    launch_bootstrap(event_proxy.clone(), bootstrap_control.clone(), generation);
                }
            }
            Event::UserEvent(UserEvent::Exit) => {
                exit_application(&bootstrap_control, &managed_process, control_flow)
            }
            Event::UserEvent(UserEvent::RefreshPage) => {
                auth_retry = None;
                let target = managed_process
                    .lock()
                    .expect("managed process lock poisoned")
                    .as_ref()
                    .and_then(DshProcess::authenticated_url)
                    .map(|url| page_navigation_target(Some(&url)).to_string())
                    .unwrap_or_else(|| page_navigation_target(None).to_string());
                let _ = desktop
                    .webview
                    .evaluate_script(&navigation_script(&target));
            }
            Event::UserEvent(UserEvent::RestartService) => {
                // 停止当前服务
                auth_retry = None;
                if let Some(process) = managed_process
                    .lock()
                    .expect("managed process lock poisoned")
                    .take()
                {
                    process.stop();
                }
                // 回到启动页，让用户看到重启过程
                let _ = desktop.splash_webview.load_html(&build_splash_html());
                let _ = desktop.webview.set_visible(false);
                let _ = desktop.splash_webview.set_visible(true);
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
                    let probe_proxy = event_proxy.clone();
                    let session = auth_retry.as_ref().map(|state| state.session);
                    let _ = desktop.webview.evaluate_script_with_callback(
                        "(document.title || '') + '\\n' + (document.body ? document.body.innerText : '')",
                        move |text| {
                            if should_hide_startup_overlay(&text) {
                                let _ = probe_proxy.send_event(UserEvent::HideStartupOverlay);
                            }
                            if let Some(session) = session {
                                let _ = probe_proxy.send_event(UserEvent::AuthPageProbe(session, text));
                            }
                        },
                    );
                }
            }
            Event::UserEvent(UserEvent::HideStartupOverlay) => {
                auth_retry = None;
                let _ = desktop.splash_webview.set_visible(false);
                let _ = desktop.webview.set_visible(true);
            }
            Event::UserEvent(UserEvent::AuthPageProbe(session, text)) => {
                if auth_retry_matches_session(auth_retry.as_ref(), session) {
                    let should_retry = auth_retry.as_ref().is_some_and(|state| {
                        should_retry_auth_page(&text, state.attempts)
                    });
                    if should_retry {
                        if let Some(state) = auth_retry.as_mut() {
                            state.attempts += 1;
                        }
                        let retry_proxy = event_proxy.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            let _ = retry_proxy.send_event(UserEvent::AuthRetry(session));
                        });
                    } else if !text.to_ascii_lowercase().contains("authentication required") {
                        auth_retry = None;
                    }
                }
            }
            Event::UserEvent(UserEvent::AuthRetry(session)) => {
                if let Some(state) = auth_retry.as_ref().filter(|state| state.session == session) {
                    let _ = desktop.webview.evaluate_script(&navigation_script(&state.url));
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
            Event::UserEvent(UserEvent::InstallDshUpdate) => {
                if dsh_update_in_progress {
                    return;
                }
                dsh_update_in_progress = true;
                auth_retry = None;
                if let Some(process) = managed_process
                    .lock()
                    .expect("managed process lock poisoned")
                    .take()
                {
                    process.stop();
                }
                let update_proxy = event_proxy.clone();
                std::thread::spawn(move || {
                    let install_ok = run_install_command("npm", &["install", "-g", "@deepseek-ai/dsh"]);
                    let validation = install_ok.then(crate::env_check::validate_dsh_installation);
                    let result = match update_action(install_ok, validation.as_ref().is_some_and(Result::is_ok)) {
                        UpdateAction::Restart => validation
                            .expect("successful installation must have validation result")
                            .map_err(|error| error),
                        UpdateAction::FailInstall => Err("dsh 更新安装失败，请稍后重试".into()),
                        UpdateAction::FailValidation => Err(validation
                            .expect("failed validation must have validation result")
                            .expect_err("failed validation must contain an error")),
                    };
                    let _ = update_proxy.send_event(UserEvent::DshUpdateFinished(result));
                });
                let _ = desktop.splash_webview.load_html(&build_splash_html());
                let _ = desktop.webview.set_visible(false);
                let _ = desktop.splash_webview.set_visible(true);
            }
            Event::UserEvent(UserEvent::DshUpdateFinished(result)) => {
                dsh_update_in_progress = false;
                match result {
                    Ok(_) => {
                        let _ = desktop.splash_webview.load_html(&build_splash_html());
                        if let Some(generation) = bootstrap_state.start() {
                            launch_bootstrap(event_proxy.clone(), bootstrap_control.clone(), generation);
                        }
                    }
                    Err(error) => {
                        let _ = desktop.splash_webview.load_html(&build_splash_html());
                        let failure_proxy = event_proxy.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            let _ = failure_proxy.send_event(UserEvent::ShowUpdateFailure(error));
                        });
                    }
                }
            }
            Event::UserEvent(UserEvent::ShowUpdateFailure(error)) => {
                let _ = desktop
                    .splash_webview
                    .evaluate_script(&format!("showUpdateFail({error:?});"));
            }
            Event::UserEvent(UserEvent::DismissDshUpdate) => {
                let _ = desktop.splash_webview.evaluate_script("dismissUpdateDot();");
            }
            Event::UserEvent(UserEvent::UpdateAvailable(true)) => {
                let _ = desktop.splash_webview.evaluate_script(
                    r#"showUpdateDot();"#,
                );
            }
            Event::UserEvent(UserEvent::UpdateAvailable(false)) => {}

            Event::UserEvent(UserEvent::InstallFinished(which, success)) => {
                if success {
                    // 自动重新检查环境并继续启动
                    if let Some(generation) = bootstrap_state.start() {
                        let _ = desktop.splash_webview.evaluate_script("reset();");
                        launch_bootstrap(event_proxy.clone(), bootstrap_control.clone(), generation);
                    }
                } else {
                    let msg = format!("{which} 安装失败，请按安装方式手动安装后重试");
                    let _ = desktop.splash_webview.evaluate_script(&format!("setStatus({msg:?});"));
                    let _ = desktop.splash_webview.evaluate_script(
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
    let main_proxy = proxy.clone();
    let webview = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_visible(false)
        .with_ipc_handler(move |request| match request.body().as_str() {
            "retry" => {
                let _ = main_proxy.send_event(UserEvent::Retry);
            }
            "exit" => {
                let _ = main_proxy.send_event(UserEvent::Exit);
            }
            "refresh" => {
                let _ = main_proxy.send_event(UserEvent::RefreshPage);
            }
            "restart" => {
                let _ = main_proxy.send_event(UserEvent::RestartService);
            }
            "toggle-exit-mode" => {
                let _ = main_proxy.send_event(UserEvent::ToggleExitOnClose);
            }
            "toggle-tray" => {
                let _ = main_proxy.send_event(UserEvent::ToggleTray);
            }
            "install-node" => {
                let _ = main_proxy.send_event(UserEvent::InstallNode);
            }
            "install-dsh" => {
                let _ = main_proxy.send_event(UserEvent::InstallDsh);
            }
            "update-dsh" => {
                let _ = main_proxy.send_event(UserEvent::InstallDshUpdate);
            }
            "dismiss-dsh-update" => {
                let _ = main_proxy.send_event(UserEvent::DismissDshUpdate);
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

    let splash_proxy = proxy.clone();
    let splash_webview = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_html(build_splash_html())
        .with_ipc_handler(move |request| match request.body().as_str() {
            "retry" => {
                let _ = splash_proxy.send_event(UserEvent::Retry);
            }
            "exit" => {
                let _ = splash_proxy.send_event(UserEvent::Exit);
            }
            "update-dsh" => {
                let _ = splash_proxy.send_event(UserEvent::InstallDshUpdate);
            }
            "dismiss-dsh-update" => {
                let _ = splash_proxy.send_event(UserEvent::DismissDshUpdate);
            }
            "install-node" => {
                let _ = splash_proxy.send_event(UserEvent::InstallNode);
            }
            "install-dsh" => {
                let _ = splash_proxy.send_event(UserEvent::InstallDsh);
            }
            _ => {}
        })
        .build(&window)
        .map_err(|error| format!("创建启动遮罩失败: {error}"))?;

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
        splash_webview,
        _web_context: web_context,
        _tray: tray,
    })
}

/// 生成导航脚本（仅导航，导航栏由 PageLoaded/InjectNavbar 注入）
fn navigation_script(target: &str) -> String {
    format!("location.replace({target:?});")
}

fn page_navigation_target(authenticated_url: Option<&str>) -> &str {
    authenticated_url.unwrap_or(DSH_URL)
}

fn should_retry_auth_page(text: &str, attempts: u8) -> bool {
    attempts == 0
        && text
            .to_ascii_lowercase()
            .contains("authentication required")
}

fn auth_retry_matches_session(state: Option<&AuthRetryState>, session: u64) -> bool {
    state.is_some_and(|state| state.session == session)
}

fn should_hide_startup_overlay(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    !normalized.is_empty() && !normalized.contains("authentication required")
}

fn update_action(install_ok: bool, validation_ok: bool) -> UpdateAction {
    if !install_ok {
        UpdateAction::FailInstall
    } else if !validation_ok {
        UpdateAction::FailValidation
    } else {
        UpdateAction::Restart
    }
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
fn inject_navbar_to_desktop(desktop: &DesktopState, settings: &Arc<Mutex<AppSettings>>) {
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

    #[test]
    fn refresh_reuses_the_authenticated_dsh_url() {
        assert_eq!(
            page_navigation_target(Some("http://127.0.0.1:3080/?token=abc")),
            "http://127.0.0.1:3080/?token=abc"
        );
        assert_eq!(page_navigation_target(None), DSH_URL);
    }

    #[test]
    fn authentication_failure_is_retried_once() {
        assert!(should_retry_auth_page("dsh web authentication required", 0));
        assert!(!should_retry_auth_page(
            "dsh web authentication required",
            1
        ));
        assert!(!should_retry_auth_page("DeepSeek web app", 0));
    }

    #[test]
    fn auth_retry_events_match_only_their_session() {
        let state = AuthRetryState {
            session: 2,
            url: "http://127.0.0.1:3080/?token=abc".into(),
            attempts: 0,
        };
        assert!(auth_retry_matches_session(Some(&state), 2));
        assert!(!auth_retry_matches_session(Some(&state), 1));
        assert!(!auth_retry_matches_session(None, 2));
    }

    #[test]
    fn dsh_update_restarts_only_after_install_and_validation() {
        assert_eq!(update_action(true, true), UpdateAction::Restart);
        assert_eq!(update_action(false, true), UpdateAction::FailInstall);
        assert_eq!(update_action(true, false), UpdateAction::FailValidation);
    }

    #[test]
    fn startup_overlay_hides_only_after_authenticated_page_load() {
        assert!(!should_hide_startup_overlay(
            "dsh web authentication required"
        ));
        assert!(!should_hide_startup_overlay(""));
        assert!(should_hide_startup_overlay("DeepSeek Harness"));
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
