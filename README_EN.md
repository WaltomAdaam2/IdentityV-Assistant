# Identity V Assistant

**Language / 语言:** [中文](README.md) | English

Identity V Assistant is a Windows desktop visual automation assistant for Identity V. It recognizes window screenshots and runs tasks through normal Windows input.

> This is a **0.1.0 development version**. Blackjack, automatic hunter match, and automatic hunter rank have passed screenshot and logic checks. As requested, no live game match was run.

## Features

* React and TypeScript desktop interface in Chinese and English, with Chinese as the default
* Tauri 2 process supervision and command bridge in Rust
* Long-running Python engine with NDJSON IPC
* Ready, heartbeat, log, state, action, metrics, and structured error events
* Pause, resume, and stop controls for login, Blackjack, automatic hunter match, and automatic hunter rank
* Pydantic-validated configuration loading and persistence
* Live Windows discovery, client rectangle, response-state, and DPI diagnostics
* Window-level capture for game windows covered by other windows or placed on another monitor
* DXcam and MSS compatibility fallbacks
* OpenCV template matching with configurable confidence
* RapidOCR recognition of the game clock in a bounded screen area
* State evidence, confirmation history, unknown-state, and recovery foundations
* Window message input for login; foreground SendInput for game tasks after restoring and focusing the game window
* Ten fixed-pixel home-screen recognition with an eight-match book click
* An “Enter the home lobby” task that resumes from any recognized login screen, confirms two frames, selects the platform, dismisses popups, and waits for home
* Blackjack resumes from a known home, matching, ready, game, or result screen, acts according to the detected role, and returns home
* Automatic hunter match enters through Diary Deduction; automatic hunter rank enters through Rank Mode and Traditional Rank. Both share hunter gameplay behavior and attempt surrender after the displayed game clock reaches six minutes
* Each game task has its own indefinite-loop option; known matching and ready screens can remain open for longer than 300 seconds
* Temporary window unresponsiveness, disappearance, or capture failure starts a 300-second recovery period; a usable frame clears it
* Independent options prevent system sleep and display sleep; the display option also keeps the session active to reduce idle locking
* Task card editors for login platform, timeout, click delays and variation, camera turn scale, and looping; changes save automatically
* Enabled task selection survives restarts; multiple enabled tasks run in list order
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

Automatic capture uses Windows Graphics Capture by default to read the game window's complete client area. Login can use window messages. Game tasks bring the game to the foreground and check focus again before input. Templates and reference screenshots are under [`assets/templates/blackjack`](assets/templates/blackjack), [`assets/templates/hunter_match`](assets/templates/hunter_match), and [`assets/templates/hunter_rank`](assets/templates/hunter_rank). Normal and ranked hunter tasks share the existing hunter HUD icon template.

## Run tasks

1. Open Identity V and this app.
2. On Tasks, use Edit to choose Android or iOS for login and set each game task's looping and shared timing options.
3. Enable the tasks you want and select Start enabled tasks. Multiple tasks run in list order.
4. The game stays in the foreground during game tasks. Pausing or stopping releases held keys.

The left sidebar shows the current stage, role, and completed cycle count. Indefinite looping is off by default. On start or resume, a task identifies the current screen and continues from a known stage. Loading does not count toward an ordinary stage timeout, but 300 seconds of continuous loading times out.

Prevent display sleep and idle lock keeps the display on and periodically sends F15 to refresh Windows' user-idle timer while the app is open. Turning the option off or closing the app stops this. Manual Win+L still locks the session, and device management policy can enforce a lock. A Windows [power request](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-powersetrequest) only keeps the display on; [idle-lock policy](https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-10/security/threat-protection/security-policy-settings/interactive-logon-machine-inactivity-limit) follows user-input inactivity.

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
uv sync --extra windows --extra ocr --extra dev
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
* Verify gameplay and results recognition in a live game environment

Process hooks, memory access, anti-cheat bypass, kernel input, and network protocol manipulation are outside this project's design.

## License

This project is licensed under the [GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`). Modified distributions must provide the corresponding source and preserve the license and copyright notices as required by the license.
