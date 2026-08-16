use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::checker;
use crate::dsh_process::DshProcess;
use crate::env_check::{self, EnvCheckResult};
use crate::run_install_command;

pub const DSH_URL: &str = "http://127.0.0.1:3080";
pub const DSH_HOST: &str = "127.0.0.1";
pub const DSH_PORT: u16 = 3080;
pub const READY_TIMEOUT: Duration = Duration::from_secs(60);
pub const READY_POLL: Duration = Duration::from_millis(500);

pub fn direct_launch_event_order() -> [&'static str; 4] {
    [
        "正在检查运行环境...",
        "正在关闭 3080 端口上的服务...",
        "正在启动 dsh web 服务...",
        "等待服务就绪...",
    ]
}

fn close_dsh_port_listeners() -> Result<(), String> {
    #[cfg(windows)]
    {
        checker::close_tcp_listeners(DSH_PORT)
    }

    #[cfg(not(windows))]
    {
        Err("仅支持 Windows".into())
    }
}

pub enum UiMsg {
    Step(String),
    EnvProgress(&'static str),
    EnvCheck(Vec<EnvCheckResult>),
    Fail(String),
    Done(DshProcess),
}

#[derive(Clone, Default)]
pub struct BootstrapControl {
    state: Arc<Mutex<BootstrapControlState>>,
}

#[derive(Default)]
struct BootstrapControlState {
    cancelled: bool,
}

pub struct SpawnPermit<'a> {
    _state: MutexGuard<'a, BootstrapControlState>,
}

impl BootstrapControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.state
            .lock()
            .expect("bootstrap control lock poisoned")
            .cancelled = true;
    }

    pub fn is_cancelled(&self) -> bool {
        self.state
            .lock()
            .expect("bootstrap control lock poisoned")
            .cancelled
    }

    pub fn try_begin_spawn(&self) -> Option<SpawnPermit<'_>> {
        let state = self.state.lock().expect("bootstrap control lock poisoned");
        if state.cancelled {
            None
        } else {
            Some(SpawnPermit { _state: state })
        }
    }
}

#[derive(Default)]
pub struct BootstrapState {
    inner: Mutex<BootstrapStateInner>,
}

#[derive(Default)]
struct BootstrapStateInner {
    active: bool,
    generation: u64,
}

impl BootstrapState {
    pub fn start(&self) -> Option<u64> {
        let mut inner = self.inner.lock().expect("bootstrap state lock poisoned");
        if inner.active {
            return None;
        }
        inner.active = true;
        inner.generation += 1;
        Some(inner.generation)
    }

    pub fn finish(&self, generation: u64) {
        let mut inner = self.inner.lock().expect("bootstrap state lock poisoned");
        if inner.generation == generation {
            inner.active = false;
        }
    }

    pub fn is_current(&self, generation: u64) -> bool {
        self.inner
            .lock()
            .expect("bootstrap state lock poisoned")
            .generation
            == generation
    }
}

pub fn run_bootstrap(tx: Sender<UiMsg>, control: &BootstrapControl) {
    if control.is_cancelled() {
        return;
    }
    send_step(&tx, direct_launch_event_order()[0]);
    // 环境检查：Node.js / npm / dsh（逐项进行，带过程动画）
    let mut results = Vec::with_capacity(3);
    for check in [env_check::check_node, env_check::check_npm, env_check::check_dsh] {
        if control.is_cancelled() {
            return;
        }
        let name = match results.len() {
            0 => "Node.js",
            1 => "npm",
            _ => "dsh (@deepseek-ai/dsh)",
        };
        let _ = tx.send(UiMsg::EnvProgress(name));
        std::thread::sleep(Duration::from_millis(350));
        results.push(check());
    }
    let _ = tx.send(UiMsg::EnvCheck(results.clone()));
    if !env_check::all_ok(&results) {
        // 环境不满足，停在检查面板等待安装后重试
        return;
    }
    if control.is_cancelled() {
        return;
    }
    send_step(&tx, direct_launch_event_order()[1]);
    if let Err(error) = close_dsh_port_listeners() {
        if !control.is_cancelled() {
            let _ = tx.send(UiMsg::Fail(error));
        }
        return;
    }
    if control.is_cancelled() {
        return;
    }
    // 关闭旧服务释放原生 DLL 后再更新 dsh。
    if let Err(error) = auto_update_dsh(&tx, &results, control) {
        if !control.is_cancelled() {
            let _ = tx.send(UiMsg::Fail(error));
        }
        return;
    }

    if control.is_cancelled() {
        return;
    }
    send_step(&tx, direct_launch_event_order()[2]);
    let permit = control.try_begin_spawn();
    let Some(_spawn_permit) = permit else {
        return;
    };
    let process = match DshProcess::spawn_dsh_web() {
        Ok(process) => process,
        Err(error) => {
            let _ = tx.send(UiMsg::Fail(format!("启动 dsh web 失败: {error}")));
            return;
        }
    };
    drop(_spawn_permit);

    if control.is_cancelled() {
        process.stop();
        return;
    }

    send_step(&tx, direct_launch_event_order()[3]);
    let mut elapsed = Duration::ZERO;
    while !checker::http_ready(DSH_HOST, DSH_PORT, READY_POLL) {
        if let Some(error) = process.exited_before_ready() {
            let _ = tx.send(UiMsg::Fail(error));
            return;
        }
        if control.is_cancelled() {
            process.stop();
            return;
        }
        std::thread::sleep(READY_POLL);
        elapsed += READY_POLL;
        if control.is_cancelled() {
            process.stop();
            return;
        }
        if elapsed >= READY_TIMEOUT {
            process.stop();
            let _ = tx.send(UiMsg::Fail("等待 dsh 服务就绪超时 (60s)".into()));
            return;
        }
    }
    if control.is_cancelled() {
        process.stop();
        return;
    }
    let _ = tx.send(UiMsg::Done(process));
}

fn send_step(tx: &Sender<UiMsg>, message: &str) {
    let _ = tx.send(UiMsg::Step(message.into()));
}

/// 对比本地与远程 dsh 版本，远程有新版本时自动安装并返回。
fn auto_update_dsh(
    tx: &Sender<UiMsg>,
    results: &[EnvCheckResult],
    control: &BootstrapControl,
) -> Result<(), String> {
    let local = results
        .iter()
        .find(|r| r.name.starts_with("dsh"))
        .and_then(|r| r.version.clone());
    let Some(local) = local else {
        return Ok(());
    };
    if control.is_cancelled() {
        return Ok(());
    }
    let Some(latest) = env_check::latest_dsh_version() else {
        return Ok(()); // 网络不可用时不阻塞启动
    };
    if control.is_cancelled() {
        return Ok(());
    }
    if latest.trim() == local.trim() {
        return Ok(());
    }
    let _ = tx.send(UiMsg::Step(
        format!("发现 dsh 新版本 {latest}，正在自动更新...").into(),
    ));
    if !run_install_command("npm", &["install", "-g", "@deepseek-ai/dsh"]) {
        return Err(format!("dsh 自动更新到 {latest} 失败"));
    }
    validate_dsh_update(&env_check::check_dsh(), &latest)
}

fn validate_dsh_update(result: &EnvCheckResult, expected: &str) -> Result<(), String> {
    if result.ok && result.version.as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(format!("dsh 更新后校验失败，期望版本 {expected}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a locally installed dsh service"]
    fn bootstrap_reaches_terminal_state() {
        let (tx, rx) = std::sync::mpsc::channel();
        let control = BootstrapControl::new();
        let worker_control = control.clone();
        let handle = std::thread::spawn(move || run_bootstrap(tx, &worker_control));
        let mut steps = 0;
        let mut terminal = None;

        for message in rx {
            match message {
                UiMsg::Step(_) => steps += 1,
                UiMsg::EnvProgress(_) | UiMsg::EnvCheck(_) => {}
                UiMsg::Fail(_) | UiMsg::Done(_) => {
                    terminal = Some(message);
                    break;
                }
            }
        }

        handle.join().unwrap();
        assert!(steps >= 1, "应至少有一个步骤");
        match terminal {
            Some(UiMsg::Done(process)) => process.stop(),
            Some(UiMsg::Fail(error)) => panic!("bootstrap 失败: {error}"),
            _ => panic!("未到达终态"),
        }
    }

    #[test]
    fn cancellation_signal_is_visible_to_bootstrap() {
        let control = BootstrapControl::new();
        assert!(!control.is_cancelled());
        control.cancel();
        assert!(control.is_cancelled());
    }

    #[test]
    fn direct_bootstrap_orders_env_check_close_spawn_then_http_wait() {
        assert_eq!(
            direct_launch_event_order(),
            [
                "正在检查运行环境...",
                "正在关闭 3080 端口上的服务...",
                "正在启动 dsh web 服务...",
                "等待服务就绪...",
            ]
        );
    }

    #[test]
    fn cancellation_waits_for_an_in_progress_spawn_decision() {
        let control = BootstrapControl::new();
        let spawn_permit = control
            .try_begin_spawn()
            .expect("spawn should be permitted before cancellation");
        let cancel_control = control.clone();
        let (cancelled_tx, cancelled_rx) = std::sync::mpsc::channel();

        let cancel_thread = std::thread::spawn(move || {
            cancel_control.cancel();
            cancelled_tx.send(()).unwrap();
        });

        assert!(
            cancelled_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "cancellation must not complete while process creation owns the decision"
        );
        drop(spawn_permit);
        cancelled_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("cancellation should complete after the spawn decision");
        cancel_thread.join().unwrap();
        assert!(control.is_cancelled());
        assert!(control.try_begin_spawn().is_none());
    }

    #[test]
    fn worker_state_rejects_retry_while_bootstrap_is_active() {
        let state = BootstrapState::default();
        let first = state.start().expect("first bootstrap should start");
        assert!(state.start().is_none());
        state.finish(first);
        assert!(state.start().is_some());
    }

    #[test]
    fn worker_state_rejects_stale_generation_events() {
        let state = BootstrapState::default();
        let first = state.start().unwrap();
        state.finish(first);
        let second = state.start().unwrap();
        assert!(!state.is_current(first));
        assert!(state.is_current(second));
    }

    #[test]
    fn dsh_update_requires_matching_healthy_version() {
        let result = EnvCheckResult {
            name: "dsh (@deepseek-ai/dsh)",
            version: Some("0.1.3".into()),
            ok: true,
            error: String::new(),
            install_hint: "",
            install_cmd: "",
        };
        assert!(validate_dsh_update(&result, "0.1.3").is_ok());
        assert!(validate_dsh_update(&result, "0.1.4").is_err());
        assert!(validate_dsh_update(
            &EnvCheckResult { ok: false, ..result },
            "0.1.3"
        )
        .is_err());
    }
}
