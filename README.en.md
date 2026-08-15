# dsh-desktop

**English** | [简体中文](README.md)

A Rust desktop wrapper that brings the [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) Web UI to Windows as a native desktop application.

## Features

- Kills any process listening on local TCP port 3080 on startup, then launches the dsh Web service in the background (default `http://127.0.0.1:3080`)
- Lives in the system tray; right-click to exit, closing the main window hides it to the tray
- Resizable main window with a minimum size of 800x600
- Auto-locates `@deepseek-ai/dsh` via `npm root -g`, with a fallback to `%APPDATA%\npm`
- Checks the runtime environment (Node.js / npm / dsh) on startup with live progress; missing components show install instructions and an auto-install button
- Checks for a newer dsh version on startup and auto-installs it before launching
- The in-app navbar provides "Restart Service" to stop and restart the dsh Web service

## Usage

1. Install Node.js (with npm) and ensure `node` is available on `PATH`.
2. Install the latest dsh globally via npm:

   ```powershell
   npm install -g @deepseek-ai/dsh
   ```

3. Build with `cargo build --release`.
4. Launch `target/release/dsh-desktop.exe`.

## Environment Check

On startup the app checks the following components one by one, showing live progress and results:

- **Node.js**: if missing, click "Auto-install" (installs the LTS build via winget) or install manually as prompted
- **npm**: ships with Node.js, no separate action needed
- **dsh (@deepseek-ai/dsh)**: if missing, click "Auto-install" (`npm install -g @deepseek-ai/dsh`) or install manually as prompted

The dsh Web service only starts after all checks pass; if anything is missing the app stays on the check panel, and you click "Retry" to continue after installing.

### Auto-Update

When dsh is already installed, startup compares against the remote version via `npm view @deepseek-ai/dsh version`. If a newer version is found it runs `npm install -g @deepseek-ai/dsh` before launching; network issues are skipped silently without blocking startup.

### Restart Service

The "Restart Service" button in the navbar stops the current dsh Web service, returns to the splash page, and re-runs the environment check and startup flow.

## Notes

- The tray and window icon uses DeepSeek's official brand assets (ICON.svg), copyright belongs to DeepSeek AI. This software merely indicates the DeepSeek Harness service it connects to and does not constitute any official endorsement, recognition, or partnership.
- dsh must be installed for the current Windows user via `npm install -g @deepseek-ai/dsh`; `npx`, project-local installs, or running from source are not supported.
- This software terminates all processes listening on local TCP port 3080. It stops the dsh Web service process it started on exit.

## License

This project is licensed under the [MIT License](LICENSE).