# Apply window settings only after enabling the engine

Status: resolved
Type: bug

## Scope and evidence

- User corrected issue 02: explicit engine start, including idle service use, must apply window settings. Ordinary game launch must not. Diagnostics may start the process without adopting layout control.
- Observed two assistants: PID 22820 started 12:03 with compiled engine PID 23828; newer assistant PID 35500 owns game PID 30172. Paths/command lines are inaccessible for these elevated processes. This is evidence of a still-running independent engine, not proof that live geometry has been repaired.
- Add an explicit layout-enable IPC command at normal engine start and F3 start. Default process/diagnostic startup leaves the window untouched; task start also enables automation layout. Reuse existing restoration on shutdown.
- Verify the established lifecycle and IPC seams under the user's recommended-default authorization. Preserve unrelated package changes and push verified checkpoints.

## Progress

- Explicit engine/F3 start now sends layout activation after readiness. Diagnostics do not send it. Task start enables layout before task work; shutdown still restores the snapshot.
- Lifecycle test failed before implementation for explicit idle engine activation, then passed. Python window/shutdown/task/IPC checks: 68 passed before adding the additional IPC validation case. Rust supervisor: 12 passed, 2 subprocess fixtures intentionally ignored. Frontend build passed.
- Old elevated assistant/engine processes remain running; source changes do not modify their loaded code. No process was forcibly terminated, and current game geometry is not claimed fixed until the old owner shuts down normally.
