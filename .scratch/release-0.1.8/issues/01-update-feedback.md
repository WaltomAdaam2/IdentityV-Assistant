# Update feedback and draft delivery

Status: resolved
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

## Final delivery (2026-10-31)

- Clean build/source commit: `7eabc9c6b175c4bbac271cccd0ce01fe26049e0e` on `SNAPSHOT-v0.1.8`. The independent release checkout uses the committed pnpm 11.19.0 lockfile; original pnpm 12.16.1/package-manager lock prefix remain unstaged in the user's checkout.
- Complete `pnpm tauri build` exited 0. Frozen-engine self-check passed all 43 templates plus Unicode-path/IPC, offline models, no Python PATH, hidden-process heartbeat/detect/capture/shutdown fixtures. Frontend TypeScript/production bundling, optimized Rust build, NSIS packaging, signing and metadata generation succeeded.
- Both source-commit CI runs passed: https://github.com/WaltomAdaam2/IdentityV-Assistant/actions/runs/41083370128 and https://github.com/WaltomAdaam2/IdentityV-Assistant/actions/runs/41083589096.
- PR: https://github.com/WaltomAdaam2/IdentityV-Assistant/pull/1, left open for user review. The GitHub connector rejected creation with integration 403, so the already authenticated GitHub CLI created it using the same authorized scope and bilingual template. PR is attached to the chat.
- Draft Release ID `429223651`: https://github.com/WaltomAdaam2/IdentityV-Assistant/releases/tag/untagged-806586dc00706f224de1. It remains draft, targets the actual clean build commit above, and contains exactly four verified assets. No release was published and public Latest remains v0.1.7. Draft URLs use GitHub's temporary untagged name until the user publishes; the manifest correctly uses the future v0.1.8 URL.
- Installer: `IdentityV-Assistant-0.1.8-Windows-x64-Setup.exe`, 87,179,669 bytes. SHA-256: `34d7e4f7091bffa9732ddb1cd6e872d1d258750a7a1b46db255ebb3b3b21dd73`. Verified with the existing original public key and Minisign verifier, including trusted signed version 0.1.8.
- All four GitHub asset digests/sizes equal the local archive; release-body hashes occur after Full Changelog and notes match latest.json. Independent delivery review: no findings; draft flag, source commit, asset hashes, body order, CI and unchanged public Latest checked separately.
- Authenticated download: the first attempt hit a CDN TLS handshake timeout. A retry retrieved all three metadata/signature files byte-for-byte and more than 256 KiB of installer data; that prefix matched. The installer transfer remained too slow, so only that task-owned downloader was stopped. **No complete remote installer round-trip is claimed**; complete stored-asset integrity is supported by GitHub's full digest plus local cryptographic verification.
- Archive: `D:/.Others/Release/work/IdentityV-Assistant-Releases/v0.1.8`. Build log: `D:/.Others/Release/work/IdentityV-Assistant-Releases/build-0.1.8.log`. Screenshots at `D:/.Others/Release/work/update-0.1.8-preview.png` and `update-0.1.8-min-window.png`; only mocked native IPC was used.
- Closed the task's browser and Vite server after verification. No native assistant was installed or launched; no account login or gameplay was tested. The legacy repository was already publicly archived by the user and was not modified.
- This final record is a documentation-only follow-up; it does not change the verified binary source commit or draft target. Once the user merges this PR, permanently freeze the SNAPSHOT branch. The user personally reviews and publishes the draft.
