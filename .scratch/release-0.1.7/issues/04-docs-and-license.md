# Bilingual public documentation and GPL-3.0-only

Status: resolved
Type: task

## Scope

- Follow WaveXinAddon: Chinese root README, README_EN.md, compact feature/install/build/credits/license sections, and separately linked detailed Chinese/English docs. Preserve the project's existing three-layer ownership and ADR.
- Apply GPL-3.0-only to current first-party source and 0.1.7; preserve historical binary and third-party notices. Use unmodified GNU GPL v3 text, not a custom license summary.

## Verified documentation corrections

- No dry-run gate exists: engine/app/engine.py constructs WindowsClientInput, and old dry_run config is migrated away. Tasks send real input.
- Capture defaults to explicit WGC. DXcam/MSS fallback occurs only for backend auto.
- Four match tasks use 300 active seconds for window recovery; login uses its configurable timeout and pauses/retains the queue on timeout.
- Hunter surrender has an OCR-unavailable fallback after 30 seconds without valid OCR and six minutes of active combat.
- Game path configuration is in launcher settings; close-to-tray is separate from normal minimize. Developer mode reveals retained detailed logs.
- Only explicit capture requests save debug frames. The save_debug_screenshots checkbox has no engine consumer and must not promise automatic failure captures.
- Developer install needs the OCR extra. IPC documentation must include actual commands/events and remove generic task_state/state_transition claims that the application does not emit.
- The visual engine uses screenshots/input; the independent login component also handles login traffic, certificates and URI registration. Documentation must distinguish them.

## Progress

- Replaced LICENSE with the official 674-line GPL v3 text downloaded from gnu.org, SHA-256 3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986.
- Confirmed KKeygen/idv-login v6.3.2-stable LICENSE is GPL v3 through the upstream repository. Added attribution and game/map rights boundaries to THIRD_PARTY_NOTICES.md.
- Rewrote both READMEs and drafted separate detailed feature docs. Link checks, remaining developer/update docs and full source consistency verification pending.

- Completed separate Chinese/English feature, development and update docs; corrected debug/IPC/architecture references. Removed the nonfunctional automatic screenshot setting while retaining manual capture and legacy config compatibility.
- Local Markdown links checked across 13 documents; source versions agree on 0.1.7. Frontend typecheck/build, 7 log tests, 60 Rust tests and 254 Python tests passed. The first sandboxed Python attempt failed on temp-folder/ONNX access; the normal-account rerun passed (cache-write warning only).
- Independent defect review found no new actionable regression. Release bridge decisions recorded in ADR 0002 and glossary.
