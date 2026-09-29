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
