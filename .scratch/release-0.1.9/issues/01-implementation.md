# Task feedback, local recovery and component progress

Status: claimed
Type: feature

## Progress

- Grilling decisions confirmed in spec; current branch SNAPSHOT-v0.1.9, original pnpm edits preserved.
- Read engine task/capture/config paths, Rust diagnostic supervision/settings rollback and React stage display. Existing login_stage needs normalization; planned_action is retrospective, not a future-action signal.
- Component screenshot root cause confirmed: ComponentUpdateDialog renders an indeterminate progress element; worker returns only a final result and Rust waits for full process output. Assistant installer progress never covered this path.
- Implemented engine preflight, active task/stage clocks, failure screenshots (default on, WGC only, 20 files / 100 MiB), settings preview/restore and bounded diagnostic exports. No new dependency. Component worker streams real per-module transfer events without replacing official apply/rollback handling.
- Independent reviewer identified and verified fixes for strict integer/window-keyword validation, excluding screen-region failure captures, autosave/navigation races, and stop-during-screenshot-save deadlock. The latter regression failed before the guard and passed afterward. Follow-up review: no findings; native dialog, shortcut-conflict rollback and actual game capture remain unexecuted.
- Offline verification: 267 engine tests passed; Rust 68 passed / 7 intentionally ignored fixture, network or native-dialog tests; frontend build/type-check and 10 helper tests passed; hotfix worker 4 passed / 1 optional official-runtime fixture skipped. Three targeted real-React/mock-IPC browser cases passed for component progress, restore/diagnostic consent and preflight/timing. Draft publisher's seven fixture gates passed.
- Visual inspection caught inherited full-width checkbox styling; reused existing SettingRow/check-label styles and allowed status guidance to wrap. Final UI recheck pending.
- Bilingual user docs, glossary and release notes updated. Checkpoint push, CI, clean signed packaging, Draft Release and PR merge remain pending.
