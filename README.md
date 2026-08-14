# dsh-desktop

A Rust desktop wrapper that brings the [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) Web UI to Windows as a native desktop application.

用 Rust 将 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) Web UI 封装为 Windows 桌面应用。

## Features / 功能

- Kills any process listening on local TCP port 3080 on startup, then launches the dsh Web service in the background (default `http://127.0.0.1:3080`)
- 每次启动时结束监听本机 TCP 端口 3080 的进程，再后台启动 dsh Web 服务（默认 `http://127.0.0.1:3080`）
- Lives in the system tray; right-click to exit, closing the main window hides it to the tray
- 系统托盘常驻，右键可退出；关闭主窗口时隐藏到托盘
- Resizable main window with a minimum size of 800x600
- 主窗口可缩放，最小尺寸为 800x600

## Usage / 使用

1. Install Node.js (with npm) and ensure `node` is available on `PATH`.
   安装 Node.js（含 npm），并确保 `node` 可从 `PATH` 使用。
2. Install the latest dsh globally via npm:
   通过 npm 全局安装最新版 dsh：

   ```powershell
   npm install -g @deepseek-ai/dsh
   ```

3. Build with `cargo build --release`.
   运行 `cargo build --release`。
4. Launch `target/release/dsh-desktop.exe`.
   启动 `target/release/dsh-desktop.exe`。

## Notes / 说明

- The tray and window icon uses DeepSeek's official brand assets (ICON.svg), copyright belongs to DeepSeek AI. This software merely indicates the DeepSeek Harness service it connects to and does not constitute any official endorsement, recognition, or partnership.
- 托盘与窗口图标使用 DeepSeek 官方品牌素材（ICON.svg），版权归 DeepSeek AI 所有。本软件仅用于指示其接入的 DeepSeek Harness 服务，不构成任何官方授权、认可或合作关系。
- This software does not install or verify Node.js, npm, or dsh dependencies. dsh must be installed for the current Windows user via `npm install -g @deepseek-ai/dsh`; `npx`, project-local installs, or running from source are not supported.
- 本软件不安装或验证 Node.js、npm 或 dsh 依赖。dsh 必须以 `npm install -g @deepseek-ai/dsh` 安装到当前 Windows 用户；不支持 `npx`、项目本地安装或源码目录启动。
- This software terminates all processes listening on local TCP port 3080. It stops the dsh Web service process it started on exit.
- 本软件会结束所有监听本机 TCP 端口 3080 的进程。退出时会结束自身启动的 dsh Web 服务进程。

## License / 许可证

This project is licensed under the [MIT License](LICENSE).
本项目遵循 [MIT License](LICENSE)。
