# dsh-desktop

**English** | [简体中文](README.zh-CN.md)

A Rust desktop wrapper that brings the [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) Web UI to Windows as a native desktop application.

## Features

- Kills any process listening on local TCP port 3080 on startup, then launches the dsh Web service in the background (default `http://127.0.0.1:3080`)
- Lives in the system tray; right-click to exit, closing the main window hides it to the tray
- Resizable main window with a minimum size of 800x600
- Auto-locates `@deepseek-ai/dsh` via `npm root -g`, with a fallback to `%APPDATA%\npm`

## Usage

1. Install Node.js (with npm) and ensure `node` is available on `PATH`.
2. Install the latest dsh globally via npm:

   ```powershell
   npm install -g @deepseek-ai/dsh
   ```

3. Build with `cargo build --release`.
4. Launch `target/release/dsh-desktop.exe`.

## Notes

- The tray and window icon uses DeepSeek's official brand assets (ICON.svg), copyright belongs to DeepSeek AI. This software merely indicates the DeepSeek Harness service it connects to and does not constitute any official endorsement, recognition, or partnership.
- This software does not install or verify Node.js, npm, or dsh dependencies. dsh must be installed for the current Windows user via `npm install -g @deepseek-ai/dsh`; `npx`, project-local installs, or running from source are not supported.
- This software terminates all processes listening on local TCP port 3080. It stops the dsh Web service process it started on exit.

## License

This project is licensed under the [MIT License](LICENSE).