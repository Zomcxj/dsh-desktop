use std::path::Path;
use std::process::{Command, Stdio};

/// 单项环境检查结果
#[derive(Clone, Debug)]
pub struct EnvCheckResult {
    pub name: &'static str,
    pub version: Option<String>,
    pub ok: bool,
    pub error: String,
    pub install_hint: &'static str,
    /// 发送给 IPC 的自动安装命令（如 "install-node" / "install-dsh"）
    pub install_cmd: &'static str,
}

const NODE_HINT: &str =
    "从 https://nodejs.org 下载 LTS 版安装，或 PowerShell 运行: winget install OpenJS.NodeJS.LTS";
const NPM_HINT: &str = "npm 随 Node.js 一起安装，重新安装 Node.js 即可";
const DSH_HINT: &str = "PowerShell 运行: npm install -g @deepseek-ai/dsh";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 运行命令并读取 stdout（Windows 下隐藏控制台窗口）。
/// npm 等是 .cmd shim，CreateProcess 无法直接启动，需经 cmd /c 包装。
fn run_output(bin: &str, args: &[&str]) -> Option<String> {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
        c.arg("/C").arg(bin).args(args);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = Command::new(bin).args(args).to_owned();

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn miss(name: &'static str, hint: &'static str, cmd: &'static str) -> EnvCheckResult {
    EnvCheckResult {
        name,
        version: None,
        ok: false,
        error: "未安装或不在 PATH 中".into(),
        install_hint: hint,
        install_cmd: cmd,
    }
}

pub fn check_node() -> EnvCheckResult {
    match run_output("node", &["--version"]) {
        Some(version) => EnvCheckResult {
            name: "Node.js",
            version: Some(version),
            ok: true,
            error: String::new(),
            install_hint: NODE_HINT,
            install_cmd: "install-node",
        },
        None => miss("Node.js", NODE_HINT, "install-node"),
    }
}

pub fn check_npm() -> EnvCheckResult {
    match run_output("npm", &["--version"]) {
        Some(version) => EnvCheckResult {
            name: "npm",
            version: Some(version),
            ok: true,
            error: String::new(),
            install_hint: NPM_HINT,
            install_cmd: "install-node",
        },
        None => miss("npm", NPM_HINT, "install-node"),
    }
}

/// 通过 `npm root -g` 找到 @deepseek-ai/dsh 的 bin.js，确认已安装
pub fn check_dsh() -> EnvCheckResult {
    let Some(root) = run_output("npm", &["root", "-g"]) else {
        return miss("dsh (@deepseek-ai/dsh)", DSH_HINT, "install-dsh");
    };
    let dsh_dir = Path::new(&root).join("@deepseek-ai").join("dsh");
    let package_json = dsh_dir.join("package.json");
    let bin_path = dsh_dir.join("lib").join("bin.js");
    if package_json.exists() && bin_path.exists() {
        let version = std::fs::read_to_string(&package_json)
            .ok()
            .and_then(|text| read_version_from_package_json(&text))
            .unwrap_or_else(|| "已安装".into());
        EnvCheckResult {
            name: "dsh (@deepseek-ai/dsh)",
            version: Some(version),
            ok: true,
            error: String::new(),
            install_hint: DSH_HINT,
            install_cmd: "install-dsh",
        }
    } else {
        miss("dsh (@deepseek-ai/dsh)", DSH_HINT, "install-dsh")
    }
}

/// 从 package.json 提取 version 字段
fn read_version_from_package_json(text: &str) -> Option<String> {
    text.lines()
        .find(|line| line.contains("\"version\""))
        .and_then(|line| line.split_once(':'))
        .and_then(|(_, value)| {
            let value = value.trim().trim_matches(',').trim_matches('"').trim();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
}

pub fn all_ok(results: &[EnvCheckResult]) -> bool {
    results.iter().all(|r| r.ok)
}

/// 从 `npm view @deepseek-ai/dsh version` 获取远程最新版本号
pub fn latest_dsh_version() -> Option<String> {
    let version = run_output("npm", &["view", "@deepseek-ai/dsh", "version"])?;
    let trimmed = version.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_environment_produces_actionable_result() {
        let result = miss("Node.js", NODE_HINT, "install-node");
        assert!(!result.ok);
        assert_eq!(result.install_cmd, "install-node");
        assert!(!result.install_hint.is_empty());
    }

    #[test]
    fn all_ok_requires_every_item() {
        let ok = EnvCheckResult {
            name: "x",
            version: None,
            ok: true,
            error: String::new(),
            install_hint: "",
            install_cmd: "",
        };
        let bad = EnvCheckResult {
            name: "y",
            version: None,
            ok: false,
            error: "未安装".into(),
            install_hint: "",
            install_cmd: "",
        };
        assert!(all_ok(&[ok.clone(), ok.clone()]));
        assert!(!all_ok(&[ok, bad]));
    }

    #[test]
    fn package_json_version_is_extracted() {
        let text = "{\n  \"name\": \"@deepseek-ai/dsh\",\n  \"version\": \"0.1.0-rc.6\",\n  \"bin\": \"lib/bin.js\"\n}";
        assert_eq!(
            read_version_from_package_json(text),
            Some("0.1.0-rc.6".into())
        );
    }

    #[test]
    fn dsh_bin_is_under_lib_dir() {
        let root = Path::new("C:\\global\\node_modules");
        let dsh_dir = root.join("@deepseek-ai").join("dsh");
        assert_eq!(dsh_dir.join("lib").join("bin.js"), {
            let _ = root;
            Path::new("C:\\global\\node_modules\\@deepseek-ai\\dsh\\lib\\bin.js")
        });
    }
}