# Release 0.1.6: close-to-tray, automatic sources and compact setup

Status: resolved
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
- Before publication, C: build cache inventory was target/debug 10.68 GiB, target/release 2.51 GiB, engine/build 0.04 GiB. Do not treat debug/components as disposable build cache. Final publication and cleanup results follow below.

## Publication and cleanup completed

- Built clean main at `21bd07898b489482c1c438c4049800c9595d8d1d` in the independent release checkout, preserving the original checkout's unrelated package.json and pnpm-lock.yaml changes.
- Published [v0.1.6](https://github.com/WaltomAdaam2/desktop-releases/releases/tag/v0.1.6) as the stable latest release with installer, signature, latest.json and SHA256SUMS.txt. Installer: 87,145,883 bytes; SHA-256 `b678265f91e77264222b515230fdc56ec71784419ea02e8a9518ef1a3d7f2bc0`.
- Build environment recovery: selected the existing Python 3.12.14 after uv's automatic interpreter download failed. After installer/signature creation, Windows PowerShell could not resolve Get-FileHash; reran only update_manifest.ps1 under PowerShell 7, then the existing publish script. No source workaround or repeated full build was needed.
- Verified the public release is not draft/prerelease, is latest, and all four remote asset digests match the stable local archive at `D:/.Others/Release/work/IdentityV-Assistant-Releases/v0.1.6`.
- Real published installer download and integrity test passed: `updates::tests::published_update_download_and_integrity`, 1 passed in 158.24 seconds. This verifies download/signature/size/hash, not installation or live account login.
- After explicit user approval and completion of the download test, removed only the original checkout's `src-tauri/target/debug/deps` and `src-tauri/target/debug/incremental`. Their measured total was 8,486,404,626 bytes (7.90 GiB). Kept release artifacts, component/runtime directories and user data.
