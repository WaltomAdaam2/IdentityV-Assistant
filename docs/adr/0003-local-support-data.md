# ADR 0003: Keep recovery and diagnostic data local and bounded

Status: accepted
Date: 2026-10-31

## Context

Version 0.1.9 adds failure screenshots, configuration recovery and diagnostic exports. Login-component account data has a separate lifecycle and must not enter assistant backups. Screen-region capture can contain other applications when the game is obscured.

## Decision

Python retains only the latest successful WGC game-window frame for automatic failure saving, recording its actual capture time. Failure images default on by user decision, with a Settings switch and 20-file / 100 MiB retention. No fresh capture or input is attempted during error cleanup.

Rust owns versioned settings validation, preview and atomic writes. Restoration holds the existing engine lifecycle lock, rejects active/queued tasks, stops an idle engine, retains the original settings and reuses desktop preference rollback. React coordinates autosave with these explicit operations.

Diagnostics collect explicit known sources instead of recursively archiving the data directory: bounded current-session logs, redacted known settings and minimal environment. Images are separately selected. Preview freezes the files that will be written to the user-selected ZIP. No automatic upload or account/runtime collection is introduced.

## Alternatives and consequences

Reusing unrestricted screen captures or archiving the entire data directory would risk unrelated personal data. A new diagnostic service or backup framework would duplicate existing process, settings and ZIP facilities. The chosen limits keep storage and exports bounded, but failure images may predate failure and text filtering cannot replace the user's review before sharing. Offline tests do not prove real capture, native dialog or game behavior.
