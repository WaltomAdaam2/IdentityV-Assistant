# Implement version 0.1.6 launcher

Status: resolved
Type: task

## Scope

Implement the full requirements in ../spec.md. Keep game/proxy state distinct from automation task state.

## Progress

- Test seams approved by user.
- Read upstream discovery, standalone parameters, process-proxy ordering, public URI interface, and launcher resource metadata.
- Need to settle runtime integration and validate announcement/background sources before implementation.
- Updated seven first-party manifest/lock versions to 0.1.6; sidebar, tooltip and About now read frontend package metadata. README version notices updated.
- Existing TypeScript compiler checked app and Vite configs successfully; parsed all seven version declarations and confirmed 0.1.6.
- pnpm wrapper attempted dependency reinstallation and stopped at a non-interactive purge prompt; used installed compiler directly without deleting/reinstalling dependencies.
- Project context milestone committed as 98d4696. Two ordinary pushes failed (GitHub connection timeout, then connection reset); push remains pending.
- User approved the shared launcher module and owned cleanup, and integration with the independent official idv-login component. Exit-while-game-running behavior is being clarified.
- Added the launcher tab between Tasks and Journal, official background/logo with recorded sources, and announcements via a bounded Rust HTTP request. Loading, error, retry, and article-open failure states are explicit.
- Navigation test was red for the absent tab and passed after implementation. Announcement failure test was red for absent error state and passed after implementation. Both browser tests now pass, including retry recovery and opening the selected official article. Each test uses a distinct browser session to avoid the CLI daemon shutdown race.
- Rust library suite: 36 passed, 6 ignored. App/Vite TypeScript checks and cargo fmt check passed. Initial rendered launcher screenshot inspected; launch button remains disabled until functional integration.
- Upstream Windows release is an installer. Runtime integration must account for required elevation, initial process-mode migration, system CA/URI registration, fixed single-instance pipe, and ownership across upstream restarts.
- User additionally requested left single/double tray clicks to restore the main window; tracked separately in issue 03.
- Independent read-only review found one missing engine handshake version. Updated ENGINE_VERSION to 0.1.6, verified it matches package metadata, and ran existing IPC tests (3 passed). No other concrete defects reported for this slice.
- Locator and direct-launch adapters implemented. A harmless compiled game fixture verifies executable, working directory, standalone arguments and child-only HTTP/HTTPS proxy; the initial direct-launch test failed before implementation.
- Proxy lifecycle uses a held native process handle and socket PID verification. Five launcher Rust tests now pass, covering valid/rejected paths, direct launch, readiness, owned-only graceful shutdown, global mode refusal, early exit and timeout cleanup.
- Browser selection tests cover persistence, rejecting a platform selection, reselecting the same valid path after an error, and recovery when that same path becomes available. Both reviewer-reported state-recovery defects were reproduced and fixed.
- Independent review found no concrete defects in the process lifecycle slice. No official component, account, certificate or system proxy was exercised by the fixtures.
- Main through 28230a1 was successfully pushed using the existing Windows proxy for that Git command only. No Git/system configuration changed.
- User confirmed exit-time cleanup of the owned component, and instructed future choices to use recommendations without waiting for further answers.

## Verification

Version/handshake metadata, both TypeScript configs, cargo fmt, Vite production build and IPC tests passed. Rust suite: 45 passed, 6 explicitly ignored. Five browser interactions pass: navigation, location/selection recovery, news failure/retry, launch status/failure recovery, and missing-component setup/selection. Inspected launcher and setup screenshots at the minimum 1000x680 window size.

The launcher is wired to both launch entry points and quit/update cleanup. A cancellation fixture proves shutdown can cancel readiness waiting. Dedicated configuration tests preserve account-shaped fixture data, disable component auto-launch, and refuse corrupted/global-recovery data. No real credentials are present in fixtures.

Independent final review reported no findings after fixing restoration when upstream partially deletes the URI registration. Windows executable build (`cargo build --bin identityv-assistant --offline -j 2`) passed. Real official component startup, certificate/URI integration, account login, physical tray focus and real update installation remain unverified; no live game/account was used. No installer was installed and no release was published.
