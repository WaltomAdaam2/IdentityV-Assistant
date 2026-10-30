# Update feedback and draft delivery

Status: claimed
Type: task

## Progress

- Confirmed main a9a2900 and only the two existing pnpm edits. GitHub had only main and no PRs. Created requested SNAPSHOT-v0.1.8 branch.
- Legacy desktop-releases is already publicly archived; its published v0.1.7 remains the final migration bridge.
- The removed feature currently probes four endpoints and caches a winner. Both update dialogs expose it; all shared Rust callers must stop probing.
- The assistant updater already emits byte counts independently of the dialog, so its progress can drive display-only rolling speed/ETA without changing transfer ownership. Component hotfix progress is indeterminate and cannot truthfully display byte speed.
- Existing release publisher ends by publishing its draft; remove that mutation and require the version's pushed SNAPSHOT branch instead.

- Implemented a five-second display-only rolling rate/ETA with 500 ms sampling, no transfer changes. Three deterministic helper tests passed (steady/stall/reset/unknown size/formatting), plus seven existing log tests.
- Removed probe/cache/race code. Rust migration maps legacy auto to direct, keeps manual routes, and rejects unknown routes. The component browser regression now verifies all four manual options, default label, and selected GHFast propagation.
- Added react-markdown 10.1.0 with raw HTML/images disabled and HTTP(S)-only links. Independent review found Windows WebView target=_blank would not open links; added a restricted native browser command and its Rust/mock-IPC tests. Final independent review: no findings.
- Real React/mock-IPC browser tests passed for safe Markdown, native-link invocation, default source, progress placement, stalled/unknown estimates, cancellation, retry and 520px layout. Screenshot at supported 1000x680 confirms visible footer controls and right-aligned speed/ETA.
- Windows PowerShell 5 publisher fixture passed seven cases: valid draft-only upload, wrong branch, dirty source, merged/frozen branch, unpushed commit, existing release, tampered artifact. No external request was made in those tests.
- Final frontend production build and Rust library suite passed (60 passed, 7 intentional fixture/live ignores). Full Python suite passed (254 tests; only a pytest-cache permission warning). Signed clean packaging, remote CI, PR and Draft Release remain.
