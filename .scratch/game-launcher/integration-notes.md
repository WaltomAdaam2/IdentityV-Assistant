# Login component integration evidence

Inspected official KKeygen/idv-login tag v6.3.2-stable on 2026-10-29/30. These are source facts and implementation decisions, not proof of live account login.

## Accepted decisions

- User approved the shared Rust launcher and an independently running official component.
- On assistant exit/update, stop only the component owned by this assistant. Preserve the game and external instances; user accepted loss of that login proxy after assistant exit.
- User is away and requested all further choices use the recommended option, without waiting for confirmation.
- Prefer an already installed official runtime, auto-discovered or manually selected. Do not silently run the release installer: it deletes old desktop links as well as its destination contents. The user subsequently requested automatic setup: scan and configure first; Official download fetches the pinned package through the existing download-source preference and extracts it into the assistant program directory, without executing installer actions.
- Keep component preferences/data separate from automation settings; avoid expanding the unrelated settings persistence refactor.

## Official runtime contract

- Release: https://github.com/KKeygen/idv-login/releases/tag/v6.3.2-stable
- Windows release EXE is an Inno installer. Actual runtime is `<root>/python-embed/python.exe <root>/src/main.pyc --proxy-port <port>`.
- The application already requests administrator elevation. Start the actual runtime directly to retain its process handle; do not use `--uri` or `--open-ui` for initial startup because they can hand off to an existing instance.
- `PROGRAMDATA` in the child selects `<PROGRAMDATA>/idv-login/config.json`. Do not read/copy user account records from another installation.
- New dedicated config needs `proxy_mode: process`, `proxy_mode_asked_0403: true`, `proxy_mode_compat_prompted_0420: true`, `hosts_cleanup_v600_done: true`, `nrpt_env_cleanup_v603_done: true`, `v6.3.2-stable_first_use: false`. The VERSION includes the v prefix, as generated from github.ref_name by build-stable.yaml.
- Host must check config writability before optionally setting `config_fixed_0403: true`; otherwise upstream's repair routine may delete an unwritable config.
- Defaults have no auto-start games. Existing dedicated config should preserve records but disable `game_settings[*].should_auto_start` and per-installation `settings.auto_start` before launch.
- Do not copy `_SAVED_PROXY_ENV`; unexpected global-proxy recovery state needs explicit error handling, not blind application to the user's environment.
- Local proxy accepts HTTP proxy requests to `http://localhost/_idv-login/health` and `/proxy-mode`. Require health success/status ok/version v6.3.2-stable and mode process, plus listening socket PID matching the held process handle.
- Official proxy can choose another port, self-update, or restart. A mismatched port or exited process is a failure, never evidence that an unrelated listener belongs to this assistant.
- Existing pipe name is `\\.\pipe\idvlogin_uri_signal`. Reject conflicting external instances rather than claiming ownership.
- Official cleanup accepts console CTRL_BREAK and has no HTTP shutdown endpoint. The managed bootstrap instead watches its private stdin pipe and calls the official cleanup function, so no console window is needed.
- Official startup still imports a root certificate, registers HKCU idvlogin URI handling, may write cloud shortcuts and game version markers. Process mode is not a promise of zero system changes. Preserve prior URI registration where possible and explain certificate behavior in setup/help.

## Current implementation and validation

- Locator checks existing absolute dwrg.exe paths and reads standard-client distribution 73 from Fever registry; excludes test-server distribution 134.
- Native proxy process uses CREATE_NO_WINDOW, a held std::process::Child handle, piped stdin/stdout/stderr and per-child PROGRAMDATA. Port ownership uses GetExtendedTcpTable.
- Stop writes to the private stdin pipe, waits for official cleanup, then falls back to terminating only the exact owned process handle. Parent pipe closure also asks the managed bootstrap to stop.
- Lifecycle fixture initially used a Windows venv redirector, which creates a separate interpreter PID. Tests now resolve sys._base_executable to match the official embedded Python process model.
- Readiness, two-instance owned-only graceful cleanup, global mode rejection, early exit and non-ready timeout cleanup tests passed. The timeout fixture allows interpreter initialization before testing a deliberately non-ready HTTP response.
- Game direct-launch fixture compiles a harmless dwrg.exe that records executable, working directory, arguments and child proxy variables. Its initial test failed before implementation and now passes.
- Initial validation used fixtures only. On 2026-10-30 the official release and unpacker were downloaded, hash verified and extracted; the real embedded runtime was used for isolated module-import/adapter checks. No account, certificate import or live game was exercised by those checks.

## Remaining implementation

- Runtime selection/version checks, dedicated configuration, prior URI restoration, shared manager, quit/update cleanup, primary launch button and setup/status states are implemented.
- Rust 45 passed / 6 ignored; five browser tests, both TypeScript configs and Vite production build passed. Minimum-window launcher and setup screenshots inspected.
- Independent final review: no remaining findings. Fixed partial upstream URI deletion recovery. No real registry or certificate changes were exercised.
- Windows executable build passed. Live official-component/account/game verification remains for the user; offline evidence does not establish login success. No installer was installed and no release was published.

## Git transport

Normal direct pushes reset. Existing Windows proxy is 127.0.0.1:7897. `git -c http.proxy=http://127.0.0.1:7897 push origin main` succeeded, without changing Git/system configuration. Main through 28230a1 is pushed.

## 2026-10-30 follow-up

- Official download v6.3.2-stable: 204067592 bytes, SHA-256 87fb99ae511dc5870a8f2d8ca6b4ac3983636521fe93db901512d054068d9083.
- Separate unpacker: jrathlev/InnoUnpacker-Windows-GUI ui_2_2_13/innounp-2.zip, 618221 bytes, SHA-256 10fce27b6344b7674d5625cc34afca1c88c12183d957c6d93e401097fa2556cd. Source/license: https://github.com/jrathlev/InnoUnpacker-Windows-GUI . It runs only after verification, in disposable staging; no upstream installer actions run.
- Actual extracted directory contains `{app}/点我启动工具.bat`, `python-embed/python.exe`, `src/main.pyc`, `src/buildinfo.pyc` and `assets`. Folder selection and discovery cover install_root.flag, D:/ProgramData/IDV-Login, C:/ProgramData/IDV-Login, Public Documents and assistant-managed components.
- A small independent Python bootstrap imports the official main under its real module name and calls main(CLI_ARGS). It suppresses startup shortcuts, self-update/hotfix overlays/restarts and the first-use account-switching wizard; it retains Qt and deliberate authentication UI. It hides certutil and turns certificate/initialization failures into logs rather than pause/input/Explorer.
- Loguru INFO/WARNING/ERROR/CRITICAL events are forwarded as bounded JSON to the assistant log under idv-login. DEBUG/ordinary stdout is drained but not imported. CRITICAL remains identified while using the existing error filter. URL query strings and common credential fields are redacted.
- Readiness is continuously checked using held process identity, exact listener ownership, health/version and process mode. A separate user-operation lock prevents health polling from spuriously rejecting a launch.
- Startup/setup can be cancelled; the main control remains clickable and shows the phase. Existing games and external instances remain outside ownership.
- Settings no longer repeats the game-path and game-launch rows; the launcher is the UI entry point.
- The user reported Chromium window-class cleanup error 1412 and pnpm ELIFECYCLE -1 on dev exit. Tauri explicitly exits with 0 after cleanup; CLI kills beforeDevCommand's process tree, which can cause nested pnpm lifecycle output. beforeDevCommand now invokes Vite directly. 1412 was separately reproduced in a native assistant with no login component. Idempotent quit plus explicit main WebView destruction before event-loop exit eliminated the warning in the follow-up native close: recorded exit code 0 and empty stderr. See issue 05.
