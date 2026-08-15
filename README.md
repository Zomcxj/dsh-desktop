# dsh-desktop

[English](README.en.md) | **简体中文**

用 Rust 将 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) Web UI 封装为 Windows 桌面应用。

## 功能

- 后台启动 dsh Web 服务（`http://127.0.0.1:3080`），启动前自动关闭占用该端口的进程
- 启动时自动检查运行环境（Node.js / npm / dsh），未安装可一键自动安装；dsh 有新版时自动更新
- 系统托盘常驻，关闭主窗口隐藏到托盘；导航栏可刷新页面、重启服务、切换退出/托盘模式

## 使用

1. 安装 Node.js（含 npm），并确保 `node` 在 `PATH` 中。
2. 全局安装 dsh：`npm install -g @deepseek-ai/dsh`
3. 运行 `cargo build --release`，启动 `target/release/dsh-desktop.exe`。

> 环境检查未通过时会停留在检查面板，安装完成后点击"重试"继续；重启服务会回到启动页重新执行检查与启动流程。

## 说明

- 托盘与窗口图标使用 DeepSeek 官方品牌素材（ICON.svg），版权归 DeepSeek AI 所有。本软件仅用于指示其接入的 DeepSeek Harness 服务，不构成任何官方授权、认可或合作关系。
- dsh 必须以 `npm install -g @deepseek-ai/dsh` 安装到当前 Windows 用户；不支持 `npx`、项目本地安装或源码目录启动。
- 本软件会结束所有监听本机 TCP 端口 3080 的进程。退出时会结束自身启动的 dsh Web 服务进程。

## 许可证

本项目遵循 [MIT License](LICENSE)。