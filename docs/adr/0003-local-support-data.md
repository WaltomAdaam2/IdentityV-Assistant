# ADR 0003: Keep recovery and diagnostic data local and bounded

Status: accepted
Date: 2026-10-31

## Context

Version 0.1.9 adds failure screenshots, configuration recovery and diagnostic exports. Login-component account data has a separate lifecycle and must not enter assistant backups. Screen-region capture can contain other applications when the game is obscured.

## Decision

Python retains only the latest successful WGC game-window frame for automatic failure saving, recording its actual capture time. Failure images default on by user decision, with a Settings switch and 20-file / 100 MiB retention. No fresh capture or input is attempted during error cleanup.

Rust owns versioned settings validation, preview and atomic writes. Restoration holds the existing engine lifecycle lock, rejects active/queued tasks, stops an idle engine, retains the original settings and reuses desktop preference rollback. React coordinates autosave with these explicit operations.

Diagnostics collect explicit known sources instead of recursively archiving the data directory: bounded current-session logs, redacted known settings and minimal environment. Images are separately selected. Preview freezes the files that will be written to the user-selected ZIP. No automatic upload or account/runtime collection is introduced.

From 0.1.11, Rust stores every journal-recognition attempt under `debug/recognition/<request-id>`, with atomic redacted reports and lossless capture, crop, processing and prepared-template images. App-session identity binds logs, screenshot ownership and reports to one running assistant. Later requests do not overwrite earlier attempts; incomplete attempts retain received evidence. Image binary stays out of text logs; a bounded JPEG crop event provides the in-progress UI preview.

Exports include only the current session, with a 100 MB whole-ZIP limit. Logs and reports precede newest complete image groups. The preview and manifest identify omissions; there is no silent image rescaling. Reports/logs expire after seven days, generated images after three days; known file names and link checks limit cleanup to owned files. A frozen preview remains unchanged by later recognition or feedback.

Manual feedback is editable local evidence, not telemetry or online learning. Offline replay consumes stored frames and exact prepared templates without accessing current map packs, windows or gameplay. It reports incomplete evidence explicitly. These choices reuse the existing log, settings, helper-process and ZIP boundaries.

## Alternatives and consequences

Reusing unrestricted screen captures or archiving the entire data directory would risk unrelated personal data. A new diagnostic service or backup framework would duplicate existing process, settings and ZIP facilities. The chosen limits keep storage and exports bounded, but failure images may predate failure and text filtering cannot replace the user's review before sharing. Offline tests do not prove real capture, native dialog or game behavior.
