# dsh-desktop

[English](README.en.md) | **简体中文**

用 Rust 将 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) Web UI 封装为 Windows 桌面应用。

## 功能

- 每次启动时结束监听本机 TCP 端口 3080 的进程，再后台启动 dsh Web 服务（默认 `http://127.0.0.1:3080`）
- 系统托盘常驻，右键可退出；关闭主窗口时隐藏到托盘
- 主窗口可缩放，最小尺寸为 800x600
- 通过 `npm root -g` 自动定位 `@deepseek-ai/dsh`，并回退到 `%APPDATA%\npm`

## 使用

1. 安装 Node.js（含 npm），并确保 `node` 可从 `PATH` 使用。
2. 通过 npm 全局安装最新版 dsh：

   ```powershell
   npm install -g @deepseek-ai/dsh
   ```

3. 运行 `cargo build --release`。
4. 启动 `target/release/dsh-desktop.exe`。

## 说明

- 托盘与窗口图标使用 DeepSeek 官方品牌素材（ICON.svg），版权归 DeepSeek AI 所有。本软件仅用于指示其接入的 DeepSeek Harness 服务，不构成任何官方授权、认可或合作关系。
- 本软件不安装或验证 Node.js、npm 或 dsh 依赖。dsh 必须以 `npm install -g @deepseek-ai/dsh` 安装到当前 Windows 用户；不支持 `npx`、项目本地安装或源码目录启动。
- 本软件会结束所有监听本机 TCP 端口 3080 的进程。退出时会结束自身启动的 dsh Web 服务进程。

## 许可证

本项目遵循 [MIT License](LICENSE)。