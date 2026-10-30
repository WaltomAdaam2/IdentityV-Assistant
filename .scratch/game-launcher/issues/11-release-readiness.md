# Release 0.1.6: close-to-tray, automatic sources and compact setup

Status: claimed
Type: task

## Scope and authorization

- With the tray preference enabled, title-bar X hides to tray; minimize stays on the taskbar. Tray Exit still quits and restores owned resources.
- Add an automatic source choice across assistant and component update/download entry points. Compare bounded parallel probes of the same public asset, considering response latency and transfer rate. Keep integrity verification and all manual choices.
- Compact launch settings so all controls fit without a vertical scrollbar at the supported minimum window size.
- User authorized publishing, followed by cache cleanup. Preserve unrelated package.json/pnpm-lock.yaml edits by using a clean release checkout. Public latest verified as v0.1.5; repository version is v0.1.6.
- Use existing desktop/update and browser test seams with recommended-default authorization. Inspect exact cache paths/sizes before proposing any recursive cleanup; retain release artifacts, signing keys and user data.

## Progress

- Close-to-tray now handles the close event, removing the old system-minimize special case and its bypass flag. Tray Exit retains the normal cleanup path.
- Automatic source resolution uses parallel bounded probes, a five-minute cache and a latency/throughput score. Manual routing and signed/hash-checked downloads are unchanged. Component checks/install and assistant downloads preserve cancellation during selection.
- Compact setup visually inspected at 1000x680; all controls fit without scrolling and the proxy pill remains visible. Long-error layout and taskbar minimize browser regressions passed.
- Rust library: 59 passed, 8 intentionally ignored fixtures/live checks. Explicit live auto-source probe passed (direct selected in this run). Frontend build passed. Python full suite: 254 passed with normal permissions; initial sandbox-only permission errors are environmental.
- C: build cache inventory: target/debug 10.68 GiB, target/release 2.51 GiB, engine/build 0.04 GiB. Do not treat debug/components as disposable build cache. Publication and exact cleanup selection remain pending.
