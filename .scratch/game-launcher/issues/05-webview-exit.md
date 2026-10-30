# WebView exit warning and dev lifecycle output

Status: resolved
Type: bug

## Report and evidence

User reported `Failed to unregister class Chrome_WidgetWin_0. Error = 1412` followed by pnpm `ELIFECYCLE -1` on exit.

A freshly compiled native assistant, with no login component started, reproduced the exact Chromium 1412 line in captured stderr on 2026-10-30. This attributes this reproduction to the assistant WebView rather than the login component. Windows 1412 means the class still has open windows. The warning alone does not establish the process exit code.

Tauri CLI terminates the beforeDevCommand process tree during normal shutdown. The nested frontend pnpm can therefore print a lifecycle failure even if the assistant exits normally. beforeDevCommand now invokes Vite directly; no exit code is forced to success and no Chromium log is filtered.

## Fix and verification

- Make quit idempotent while asynchronous service cleanup runs.
- After cleanup, destroy the main WebView window before requesting event-loop exit. Do not call close(), which would reenter CloseRequested.
- Windows executable rebuilt successfully; independent review found no defects in the ordering/ownership change.
- User-assisted native close after the fix returned exit code 0 and an empty stderr file (0 bytes); the previously reproduced 1412 warning did not recur. No login component/game was active during this exit test. The attempted temporary local WebView debugging port was not available; no permanent debugging setting was added.

## Development startup regression (2026-10-30)

- The direct-Vite change incorrectly specified a relative `cwd: frontend` in the hook object. The user's complete `pnpm tauri dev` invocation failed with Windows error 267 before starting Vite. The earlier production build and native EXE launch did not cover this development entry point; reporting the whole startup change as verified was incorrect.
- Removed the working-directory override. The hook now runs `node frontend/node_modules/vite/bin/vite.js frontend` from Tauri's default frontend directory (the repository root), passing the Vite project root explicitly.
- Executed the exact `pnpm tauri dev` command with normal host permissions. Stopped the old test Vite server, confirmed the new server alone listening on `[::1]:1420` (PID 41276), HTTP 200 from the development URL, Rust compilation completion and a newly started native assistant (PID 37012) with its main window present. The development session remains running for the user.
- Preserved the separately validated WebView exit fix and the user's unrelated package-manager changes.
