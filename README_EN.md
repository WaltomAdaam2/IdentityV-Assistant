# Identity V Assistant

**Language / 语言:** [中文](README.md) | English

Identity V Assistant is a Windows desktop framework for external visual automation of the PC version of Identity V. It organizes workflows around visible screen recognition, a state machine, and normal Windows input. It does not read or modify game process memory, inject DLLs, or manipulate network traffic.

> The project is currently at the **0.1.0 framework stage**. Full match automation, weekly farming, character logic, and pathfinding are not implemented.

## Features

* React and TypeScript desktop utility interface in Simplified Chinese
* Tauri 2 process supervision and command bridge in Rust
* Long-running Python engine with NDJSON IPC
* Ready, heartbeat, log, state, action, metrics, and structured error events
* Pause, resume, and stop controls for a safe MockTask
* Pydantic-validated configuration loading and persistence
* Windows discovery, client rectangle, and DPI diagnostics
* DXcam capture with MSS fallback
* OpenCV template matching with configurable confidence
* Lazy OCR interface with an optional RapidOCR backend
* State evidence, confirmation history, unknown-state, and recovery foundations
* Global dry run, SendInput abstraction, and held-input cleanup
* Debug, log filtering, settings, and developer input-test pages
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
visible window + normal Windows input
```

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

* Collect legal game screenshots and templates
* Implement and tune real state detectors
* Add verified semantic actions for the weekly task
* Package the Python engine as a standalone executable
* Add the OCR backend and real-environment integration tests

Process hooks, memory access, anti-cheat bypass, kernel input, and network protocol manipulation are outside this project's design.

## License

This project is licensed under the [GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`). Modified distributions must provide the corresponding source and preserve the license and copyright notices as required by the license.
