# Implement version 0.1.6 launcher

Status: claimed
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

## Verification

Version/handshake metadata, TypeScript, cargo fmt, browser navigation/news tests, Rust library tests and IPC tests passed. Independent review findings resolved. Game discovery, proxy integration and live Windows/game verification remain pending.
