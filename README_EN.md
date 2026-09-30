# Identity V Assistant

**Language / 语言:** [中文](README.md) | English

Identity V Assistant is a Windows desktop visual automation assistant for Identity V. It recognizes window screenshots and runs tasks through normal Windows input.

> This is a **0.1.0 development version**. The Blackjack flow has passed screenshot recognition and timing checks, but a full match has not been verified in the live game.

## Features

* React and TypeScript desktop interface in Chinese and English, with Chinese as the default
* Tauri 2 process supervision and command bridge in Rust
* Long-running Python engine with NDJSON IPC
* Ready, heartbeat, log, state, action, metrics, and structured error events
* Pause, resume, and stop controls for the “Enter the home lobby” and “Blackjack” tasks
* Pydantic-validated configuration loading and persistence
* Live Windows discovery, client rectangle, response-state, and DPI diagnostics
* Window-level capture for game windows covered by other windows or placed on another monitor
* DXcam and MSS compatibility fallbacks
* OpenCV template matching with configurable confidence
* Lazy OCR interface with an optional RapidOCR backend
* State evidence, confirmation history, unknown-state, and recovery foundations
* Window message input for login; foreground SendInput for Blackjack after restoring and focusing the game window
* Ten fixed-pixel home-screen recognition with an eight-match book click
* An “Enter the home lobby” task that resumes from any recognized login screen, confirms two frames, selects the platform, dismisses popups, and waits for home
* A Blackjack task that opens the book from home, selects the mode, matches and readies up, acts according to the detected role, and returns home after the result screen; optional indefinite looping
* Task card editors for login platform, timeout, click delays and variation, camera turn scale, and looping; changes save automatically
* Enabled task selection survives restarts; enabled tasks run in list order
* Optional memory protection that collects unused objects above 90% system memory use
* Capture diagnostics, concise logs, and window settings available to regular users; logs support search, filtering, pagination, and page jumping
* Python and Rust core test foundations

## Architecture

```text
React / TypeScript GUI
        |
        | Tauri commands + events
        v
Rust lifecycle supervisor
        |
        | NDJSON over stdin/stdout
        v
Python visual automation engine
        |
        v
window capture + Windows input
```

Automatic capture uses Windows Graphics Capture by default to read the game window's complete client area. Login can use window messages. Blackjack brings the game to the foreground and checks focus again before input. Templates and supplied reference screenshots are together in [`assets/templates/blackjack`](assets/templates/blackjack).

## Run tasks

1. Open Identity V and this app.
2. On Tasks, use Edit to choose Android or iOS for login and tune Blackjack settings.
3. Enable the tasks you want and select Start enabled tasks. When both are enabled, login runs before Blackjack.
4. The game stays in the foreground during Blackjack. Pausing or stopping releases held keys.

The left sidebar shows the current stage, role, and completed cycle count. Indefinite looping is off by default. Loading does not count toward an ordinary stage timeout, but 300 seconds of continuous loading times out.

## Local data

Settings and runtime data are stored under `%LOCALAPPDATA%\IdentityV-Assistant\`:

```text
config.json                  Settings and enabled tasks
logs\YYYY-MM-DD.jsonl       Complete daily event archive
debug\captures\            Manual diagnostic screenshots
```

Daily logs are kept for 30 days. Expired files are removed on startup and when the date changes. The Logs page shows only the current app session, retaining at most 99,000 entries with 1,000 per page. Clearing the page does not delete daily files. Verbose mode changes only what the page shows; disk logs remain complete.

See [Architecture](docs/architecture.md) for layer boundaries and [IPC Protocol](docs/ipc.md) for message definitions.

## Requirements

* Windows 11 x86_64
* Node.js LTS and pnpm 11+
* Rust stable with the MSVC toolchain
* Python 3.12+
* uv (recommended)
* WebView2 Runtime

## Install Dependencies

```powershell
pnpm install
Set-Location engine
uv sync --extra windows --extra dev
Set-Location ..
```

## Run

```powershell
pnpm tauri dev
```

Run only the frontend:

```powershell
pnpm dev
```

Run only the Python protocol engine:

```powershell
Set-Location engine
uv run python main.py
```

## Verification

```powershell
pnpm typecheck
pnpm build

Set-Location engine
uv run pytest

Set-Location ..\src-tauri
cargo fmt --check
cargo test
```

## Project Layout

```text
frontend/       React GUI
src-tauri/      Tauri shell and Python process supervisor
engine/         Python automation engine
assets/         Template and icon registry content
config/         Default configuration
docs/           Architecture and development documentation
scripts/        Windows development helpers
```

## Planned Work

* Calibrate screen recognition and input against live game sessions
* Package the Python engine as a standalone executable
* Add the OCR backend and real-environment integration tests

Process hooks, memory access, anti-cheat bypass, kernel input, and network protocol manipulation are outside this project's design.

## License

This project is licensed under the [GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`). Modified distributions must provide the corresponding source and preserve the license and copyright notices as required by the license.
