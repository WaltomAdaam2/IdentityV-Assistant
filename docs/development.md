# Development

## Requirements

- Windows 11 x86_64
- Node.js LTS and pnpm
- Rust stable with the MSVC toolchain
- Python 3.12 or newer
- uv (recommended)
- Microsoft Edge WebView2 runtime

## Install

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

Run the protocol engine directly:

```powershell
Set-Location engine
uv run python main.py
```

Each stdout line is NDJSON. Enter a command such as `{"version":1,"type":"ping","timestamp":"2026-01-01T00:00:00Z","request_id":"manual"}`.

## Task lifecycle checks

Use `start_task` or `start_tasks` through the Tauri command bridge when exercising the desktop application. Those commands start the Python service automatically if it is stopped. `start_tasks` retains the supplied ordering. F3 is a global pause/resume shortcut for an active task; pausing releases held input and freezes task timers.

For a nonrecoverable task failure, confirm that the active task and pending queue move to paused status, then resume manually. Resume reconstructs the task and routes from a fresh capture. Stop clears the active task and all pending entries.

The repository's automated checks use supplied screenshots, fixtures, and mocked input paths. No live Identity V game session has been run for this release; do not treat passing local checks as live gameplay validation.

## Checks

```powershell
pnpm typecheck
pnpm build
Set-Location engine
uv run pytest
Set-Location ..\src-tauri
cargo fmt --check
cargo test
```

Set `IDENTITYV_PYTHON` to an explicit Python executable and `IDENTITYV_ENGINE_PATH` to `main.py` when automatic development path discovery is unsuitable.
