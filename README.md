# dsh-desktop

[English](README.en.md) | **简体中文**

用 Rust 将 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) Web UI 封装为 Windows 桌面应用。

## 功能

- 每次启动时结束监听本机 TCP 端口 3080 的进程，再后台启动 dsh Web 服务（默认 `http://127.0.0.1:3080`）
- 系统托盘常驻，右键可退出；关闭主窗口时隐藏到托盘
- 主窗口可缩放，最小尺寸为 800x600
- 通过 `npm root -g` 自动定位 `@deepseek-ai/dsh`，并回退到 `%APPDATA%\npm`
- 启动时自动检查运行环境（Node.js / npm / dsh）并显示版本，未安装时提供安装命令与自动安装按钮
- 启动时自动检查 dsh 新版本，发现更新则自动安装后再启动
- 界面导航栏提供"重启服务"，可停止并重新启动 dsh Web 服务

## 使用

1. 安装 Node.js（含 npm），并确保 `node` 可从 `PATH` 使用。
2. 通过 npm 全局安装最新版 dsh：

   ```powershell
   npm install -g @deepseek-ai/dsh
   ```

3. 运行 `cargo build --release`。
4. 启动 `target/release/dsh-desktop.exe`。

## 环境检查

启动时会依次检查以下环境，面板实时显示检查进度与结果：

- **Node.js**：未安装时可点击"自动安装"（通过 winget 安装 LTS 版），或按提示手动安装
- **npm**：随 Node.js 一起安装，无需单独处理
- **dsh (@deepseek-ai/dsh)**：未安装时可点击"自动安装"（`npm install -g @deepseek-ai/dsh`），或按提示手动安装

全部通过后才会启动 dsh Web 服务；任一项未安装会停留在检查面板，安装完成后点击"重试"继续。

### 自动更新

dsh 已安装时，启动过程会通过 `npm view @deepseek-ai/dsh version` 对比远程版本。发现新版本会自动执行 `npm install -g @deepseek-ai/dsh` 更新后再启动；网络不可用时静默跳过，不阻塞启动。

### 重启服务

导航栏的"重启服务"按钮会停止当前 dsh Web 服务并回到启动页重新执行环境检查与启动流程。

## 说明

- 托盘与窗口图标使用 DeepSeek 官方品牌素材（ICON.svg），版权归 DeepSeek AI 所有。本软件仅用于指示其接入的 DeepSeek Harness 服务，不构成任何官方授权、认可或合作关系。
- dsh 必须以 `npm install -g @deepseek-ai/dsh` 安装到当前 Windows 用户；不支持 `npx`、项目本地安装或源码目录启动。
- 本软件会结束所有监听本机 TCP 端口 3080 的进程。退出时会结束自身启动的 dsh Web 服务进程。

## 许可证

本项目遵循 [MIT License](LICENSE)。