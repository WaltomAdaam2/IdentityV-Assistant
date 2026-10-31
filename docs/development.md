# Development

**Language:** [中文](development_zh.md) | English · [README](../README_EN.md)

## Requirements

Windows 11 x64, Node.js 24, the pnpm version declared in the root `package.json`, Rust stable/MSVC with Visual Studio C++ build tools, Python 3.12+, uv, and Microsoft Edge WebView2. The build scripts install Python build dependencies and bundle a standalone engine; end users do not need Python.

```powershell
pnpm install --frozen-lockfile
uv sync --project engine --extra windows --extra ocr --extra dev --extra build
pnpm tauri dev
```

Development mode launches the real desktop application. Starting a task sends real game input; there is no default dry run. Set `IDENTITYV_PYTHON` to an explicit Python executable and `IDENTITYV_ENGINE_PATH` to `main.py` if automatic discovery is unsuitable.

The engine can also be started directly with `uv run --project engine python engine/main.py`. Its stdin/stdout use [NDJSON](ipc.md), with one command per line, for example:

```json
{"version":1,"type":"ping","timestamp":"2026-01-01T00:00:00Z","request_id":"manual"}
```

## Checks

```powershell
pnpm typecheck
pnpm build
node --test frontend/tests/logs.test.mjs
uv run --project engine pytest engine/tests
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```

The frontend browser tests use the real React UI with mocked Tauri IPC. See `frontend/tests/launcher.test.mjs`: start Vite on port 1420 and provide the `agent-browser` executable through `AGENT_BROWSER_BIN` if it is not on PATH. These tests are separate from native dialogs, installation, login, and gameplay. Do not treat fixture-based recognition, mocked input, or a successful build as live game validation.

## Ownership and lifecycle

- React edits settings and displays events; game decisions stay in Python.
- Rust owns processes, configuration, map imports, update validation, and application shutdown.
- Python owns capture, recognition, task routing, timers, and guarded input.

`start_task` / `start_tasks` start a stopped engine automatically. F3 toggles the engine, serialized with other lifecycle commands. Explicit automation startup applies the configured game window layout; diagnostics do not. Pause releases input and freezes task timers. Resume captures a fresh screen. Stop clears the queue. A recoverable foreground/capture loss receives a bounded recovery period; see [feature logic](README_LOGIC_EN.md) and [IPC](ipc.md).

Keep local work specifications and issues under `.scratch/` according to [the issue workflow](agents/issue-tracker.md). Record actual architectural decisions in `docs/adr/` and shared terms in `GLOSSARY.md`; avoid broad rewrites for local changes.

## Packaging and publishing

Use `pnpm tauri build` for a signed installer. Packaging alone never publishes a release. The signing key, clean-source checks, asset names, and release procedure are documented separately in [Updates and releases](updates_EN.md).
