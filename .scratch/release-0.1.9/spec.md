# Version 0.1.9: task feedback and local recovery

Status: claimed

## Confirmed scope

- User confirmed all five selected features through grilling on 2026-10-31: task stage/timing/reason/guidance, failure screenshots, task-start preflight, configuration backup/restore, and diagnostic bundle export. Map package management is excluded.
- Failure screenshots default ON (explicit user choice), at most one per failure, last 20 files and 100 MiB combined. Local game client images only; show actual capture time, never claim a stale frame is the failure instant. Provide an off switch.
- Diagnostics default to versions, necessary environment, current-session bounded logs and sanitized settings. Preview file names/sizes. Screenshots are separately selected; exclude login account data, credentials, signing keys, maps and component runtime. Never upload automatically.
- Settings backup covers assistant preferences and task parameters, not account data/maps/logs. Validate external input and schema/version, preview differences, restore whole settings after confirmation, preserve the old settings and roll back failures. Block restoration during running/paused tasks; validate machine paths and avoid autosave races.
- Automatically preflight new user-started tasks/queues; success continues, missing/unresponsive windows or failed capture blocks with recheck/diagnostic guidance. Resolution/focus differences do not block before existing layout/focus handling. Do not repeat dialogs per match or introduce automatic backend switching.
- Reuse task_progress and sidebar UI. Display effective task/stage duration excluding pauses, with stage-level next-step guidance; reset on engine restart rather than inventing persisted timings.
- Additional user report: login-component hotfix dialog lacks real speed and byte counts. Extend component progress across worker/Rust/UI; report actual download bytes/rate where available and explicit non-download phases, never fabricated totals/ETA.

## Delivery and constraints

- New SNAPSHOT-v0.1.9 from verified main 68a3188. Keep original package.json pnpm@12.16.1 and pnpm-lock.yaml 183-line local edits outside commits.
- Verified checkpoint pushes, independent review, PR merge when checks pass, then permanent frozen branch; signed Draft Release only, user publishes personally.
- Builds and offline/mock-IPC browser checks only. Do not install/run native assistant or operate game/account.
- Use existing Python task/capture, Rust lifecycle/settings/ZIP, and React presentation seams; no new dependencies or broad rewrite.

## Verification

Behavior tests for preflight success/failure/no input, stage timing/pause reset, screenshot opt-out/retention/save failures; Rust tests for backup validation/rollback and sanitized bounded bundle contents; mocked browser tests for status/error actions, export/restore preview and real component progress. Full relevant suites, CI, signed packaging, and independent review before merge/draft delivery.

## Issues

- [Implementation and delivery](issues/01-implementation.md)
