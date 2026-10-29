# Login component integration evidence

Inspected official KKeygen/idv-login tag v6.3.2-stable on 2026-10-29/30. These are source facts and implementation decisions, not proof of live account login.

## Accepted decisions

- User approved the shared Rust launcher and an independently running official component.
- On assistant exit/update, stop only the component owned by this assistant. Preserve the game and external instances; user accepted loss of that login proxy after assistant exit.
- User is away and requested all further choices use the recommended option, without waiting for confirmation.
- Prefer an already installed official runtime, auto-discovered or manually selected. Do not silently run the release installer: it deletes old desktop links as well as its destination contents. Provide the official release link when missing.
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
- Official cleanup accepts console CTRL_BREAK. There is no HTTP shutdown endpoint.
- Official startup still imports a root certificate, registers HKCU idvlogin URI handling, may write cloud shortcuts and game version markers. Process mode is not a promise of zero system changes. Preserve prior URI registration where possible and explain certificate behavior in setup/help.

## Current implementation and validation

- Locator checks existing absolute dwrg.exe paths and reads standard-client distribution 73 from Fever registry; excludes test-server distribution 134.
- Native proxy process has its own hidden console, held process handle and per-child PROGRAMDATA. Port ownership uses GetExtendedTcpTable.
- A short-lived Python signal process explicitly FreeConsole, AttachConsole to the owned private console, sends CTRL_BREAK, and detaches. This does not alter the assistant console. Graceful wait has an exact-handle termination fallback.
- Lifecycle fixture initially used a Windows venv redirector, which creates a separate interpreter PID. Tests now resolve sys._base_executable to match the official embedded Python process model.
- Readiness, two-instance owned-only graceful cleanup, global mode rejection, early exit and non-ready timeout cleanup tests passed. The timeout fixture allows interpreter initialization before testing a deliberately non-ready HTTP response.
- Game direct-launch fixture compiles a harmless dwrg.exe that records executable, working directory, arguments and child proxy variables. Its initial test failed before implementation and now passes.
- No official component has been downloaded/executed, no account used, and no certificate or system proxy changed during these tests.

## Remaining implementation

- Runtime selection/version checks, dedicated configuration, prior URI restoration, shared manager, quit/update cleanup, primary launch button and setup/status states are implemented.
- Rust 45 passed / 6 ignored; five browser tests, both TypeScript configs and Vite production build passed. Minimum-window launcher and setup screenshots inspected.
- Independent final review: no remaining findings. Fixed partial upstream URI deletion recovery. No real registry or certificate changes were exercised.
- Windows executable build passed. Live official-component/account/game verification remains for the user; offline evidence does not establish login success. No installer was installed and no release was published.

## Git transport

Normal direct pushes reset. Existing Windows proxy is 127.0.0.1:7897. `git -c http.proxy=http://127.0.0.1:7897 push origin main` succeeded, without changing Git/system configuration. Main through 28230a1 is pushed.
