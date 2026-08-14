use std::env;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Clone)]
pub struct DshProcess {
    pub managed: bool,
    pub pid: Option<u32>,
    child: Arc<Mutex<Option<Child>>>,
    stderr_reader: Arc<Mutex<Option<JoinHandle<String>>>>,
    stdout_reader: Arc<Mutex<Option<JoinHandle<String>>>>,
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;

impl DshProcess {
    pub fn spawn_dsh_web() -> std::io::Result<Self> {
        #[cfg(windows)]
        use std::os::windows::process::CommandExt;

        let appdata = env::var_os("APPDATA")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未设置 APPDATA 环境变量"))?;
        let mut command = dsh_web_command(Path::new(&appdata));
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)]
        command.creation_flags(node_launch_flags());

        let mut child = command.spawn()?;
        let pid = child.id();
        let stdout_reader =
            drain_output(child.stdout.take().expect("stdout was configured as piped"));
        let stderr_reader =
            drain_output(child.stderr.take().expect("stderr was configured as piped"));
        Ok(Self {
            managed: true,
            pid: Some(pid),
            child: Arc::new(Mutex::new(Some(child))),
            stderr_reader: Arc::new(Mutex::new(Some(stderr_reader))),
            stdout_reader: Arc::new(Mutex::new(Some(stdout_reader))),
        })
    }

    pub fn exited_before_ready(&self) -> Option<String> {
        let exited = self
            .child
            .lock()
            .expect("dsh child lock poisoned")
            .as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_some());
        if !exited {
            return None;
        }

        let stderr = self.collect_output(&self.stderr_reader);
        let _ = self.collect_output(&self.stdout_reader);
        Some(child_exited_before_ready(&stderr))
    }

    pub fn stop(&self) {
        if !self.managed {
            return;
        }

        if let Some(pid) = self.pid {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }

    fn collect_output(&self, reader: &Arc<Mutex<Option<JoinHandle<String>>>>) -> String {
        reader
            .lock()
            .expect("dsh output reader lock poisoned")
            .take()
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default()
    }
}

fn dsh_web_command(appdata: &Path) -> Command {
    let bin = resolve_dsh_bin(appdata);
    let mut command = Command::new("node");
    command.arg(bin).arg("web");
    command
}

/// 查找 `@deepseek-ai/dsh` 的 bin.js，按优先级：
///
/// 1. `npm root -g` 输出的全局 node_modules 路径
/// 2. 默认 `%APPDATA%\npm\node_modules` 路径（fallback）
fn resolve_dsh_bin(appdata: &Path) -> PathBuf {
    let candidates = [
        npm_global_root().map(|root| {
            root.join("@deepseek-ai")
                .join("dsh")
                .join("lib")
                .join("bin.js")
        }),
        Some(
            appdata
                .join("npm")
                .join("node_modules")
                .join("@deepseek-ai")
                .join("dsh")
                .join("lib")
                .join("bin.js"),
        ),
    ];

    candidates
        .into_iter()
        .flatten()
        .find(|p| p.exists())
        .unwrap_or_else(|| {
            // 全部不存在时走默认 fallback，让 Node.js 报错
            appdata
                .join("npm")
                .join("node_modules")
                .join("@deepseek-ai")
                .join("dsh")
                .join("lib")
                .join("bin.js")
        })
}

/// 通过 `npm root -g` 获取全局 node_modules 目录。
fn npm_global_root() -> Option<PathBuf> {
    // Windows 上 npm 是 .cmd shim，CreateProcess 无法直接启动，需经 cmd /c
    #[cfg(windows)]
    let mut command = {
        let mut c = Command::new("cmd");
        c.args(["/C", "npm", "root", "-g"]);
        c
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut c = Command::new("npm");
        c.args(["root", "-g"]);
        c
    };

    let output = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(windows)]
fn node_launch_flags() -> u32 {
    CREATE_NO_WINDOW | DETACHED_PROCESS
}

fn child_exited_before_ready(stderr: &str) -> String {
    let stderr = stderr.trim();
    if stderr.is_empty() {
        "dsh web 在服务就绪前退出".into()
    } else {
        format!("dsh web 在服务就绪前退出: {stderr}")
    }
}

fn drain_output<R: Read + Send + 'static>(mut output: R) -> JoinHandle<String> {
    thread::spawn(move || {
        let mut text = String::new();
        let _ = output.read_to_string(&mut text);
        text
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checker;
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn dsh_web_command_runs_node_bin_without_cmd() {
        let command = dsh_web_command(Path::new(r"C:\Users\test\AppData\Roaming"));

        assert_eq!(command.get_program(), "node");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args.len(), 2, "应有两个参数: bin.js 路径和 'web'");
        assert!(
            args[0].to_string_lossy().ends_with("bin.js"),
            "第一个参数应以 bin.js 结尾: {}",
            args[0].to_string_lossy()
        );
        assert_eq!(args[1], "web");
        assert_ne!(command.get_program(), "cmd");
    }

    #[test]
    fn resolve_dsh_bin_finds_installed_package() {
        let appdata = Path::new(r"C:\Users\test\AppData\Roaming");
        let bin = resolve_dsh_bin(appdata);

        // 如果系统上安装了 @deepseek-ai/dsh，应找到真实路径；
        // 否则回退到默认路径
        assert!(
            bin.to_string_lossy().contains("bin.js"),
            "应返回 bin.js 路径: {}",
            bin.display()
        );
    }

    #[test]
    fn npm_global_root_returns_some_path() {
        let root = npm_global_root();
        assert!(root.is_some(), "npm root -g 应返回有效路径");
        if let Some(root) = root {
            assert!(
                root.to_string_lossy().contains("node_modules"),
                "全局根应包含 node_modules: {}",
                root.display()
            );
        }
    }

    #[test]
    fn early_child_exit_is_a_user_facing_failure() {
        let failure = child_exited_before_ready("Error: address already in use");

        assert!(failure.contains("服务就绪前退出"));
        assert!(failure.contains("Error: address already in use"));
    }

    #[cfg(windows)]
    #[test]
    fn node_launch_detaches_from_the_console() {
        assert_eq!(node_launch_flags() & DETACHED_PROCESS, DETACHED_PROCESS);
    }

    #[test]
    #[ignore = "starts a real local dsh service"]
    fn spawn_starts_service_on_3080() {
        if checker::http_ready("127.0.0.1", 3080, Duration::from_millis(300)) {
            return;
        }

        let process = DshProcess::spawn_dsh_web().expect("启动 dsh web");
        assert!(process.pid.is_some());
        let mut ready = false;
        for _ in 0..60 {
            if checker::http_ready("127.0.0.1", 3080, Duration::from_millis(500)) {
                ready = true;
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        assert!(ready, "dsh web 未在 60s 内就绪");
        process.stop();
        std::thread::sleep(Duration::from_millis(1500));
        assert!(!checker::http_ready(
            "127.0.0.1",
            3080,
            Duration::from_millis(300)
        ));
    }

    #[test]
    fn unmanaged_process_is_not_killed() {
        let process = DshProcess {
            managed: false,
            pid: Some(99999),
            child: Arc::new(Mutex::new(None)),
            stderr_reader: Arc::new(Mutex::new(None)),
            stdout_reader: Arc::new(Mutex::new(None)),
        };
        process.stop();
    }
}
