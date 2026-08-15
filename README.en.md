# dsh-desktop

**English** | [简体中文](README.md)

A Rust desktop wrapper that brings the [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) Web UI to Windows as a native desktop application.

## Features

- Launches the dsh Web service in the background (`http://127.0.0.1:3080`), closing any process already listening on that port first
- Checks the runtime environment (Node.js / npm / dsh) on startup; missing components can be auto-installed, and newer dsh releases are auto-installed before launch
- Lives in the system tray; closing the main window hides it to the tray. The navbar can refresh the page, restart the service, or toggle exit/tray modes

## Usage

1. Install Node.js (with npm) and ensure `node` is available on `PATH`.
2. Install dsh globally: `npm install -g @deepseek-ai/dsh`
3. Build with `cargo build --release`, then launch `target/release/dsh-desktop.exe`.

> If the environment check fails, the app stays on the check panel — click "Retry" to continue after installing. "Restart Service" returns to the splash page and re-runs the check and startup flow.

## Notes

- The tray and window icon uses DeepSeek's official brand assets (ICON.svg), copyright belongs to DeepSeek AI. This software merely indicates the DeepSeek Harness service it connects to and does not constitute any official endorsement, recognition, or partnership.
- dsh must be installed for the current Windows user via `npm install -g @deepseek-ai/dsh`; `npx`, project-local installs, or running from source are not supported.
- This software terminates all processes listening on local TCP port 3080. It stops the dsh Web service process it started on exit.

## License

This project is licensed under the [MIT License](LICENSE).