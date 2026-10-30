# IPC protocol

Protocol version: `1`. Transport: one UTF-8 JSON object per line over Python stdin/stdout. Python stdout is reserved for protocol messages; raw diagnostics use stderr.

Every message includes `version`, `type`, and an ISO-8601 `timestamp`. Commands also include a `request_id`; the Rust bridge supplies missing envelope fields.

## Rust to Python

| Type | Main fields | Purpose |
| --- | --- | --- |
| `start_task` | `task` | Start one registered task |
| `start_tasks` | ordered `tasks` array | Run enabled tasks in list order |
| `pause_task` | — | Pause the active task, release held input, and freeze its task timers |
| `resume_task` | — | Resume the active task and reroute from a fresh screen capture |
| `stop_task` | — | Cancel the active task, release held inputs, and clear the remaining queue |
| `ping` | — | Request a liveness response |
| `shutdown` | — | Graceful engine shutdown |
| `detect_window` | — | Run window diagnostics |
| `capture_frame` | — | Save one debug frame |
| `test_template` | `template` | Match one registered template |
| `input_test` | `action` | Developer input test; first requires an explicit `set_enabled` action |
| `reload_settings` | — | Reload the saved configuration |
| `enable_window_layout` | — | Enable game window layout for explicit automation startup |

## Python to Rust

| Type | Main fields | Purpose |
| --- | --- | --- |
| `ready` | `engine_version` | Engine initialized |
| `heartbeat` | `engine_status`, `task_status` | Liveness and lifecycle |
| `planned_action` | `action`, `input`, `target` | Action sent to the game |
| `task_started` | `task_id` | Task selected from the queue |
| `task_progress` | `task_id`, `stage`, `role`, `completed_cycles`, `infinite_loop` | Current task progress |
| `task_paused` | `task_id`, `reason`, `message` | User pause or a paused nonrecoverable task failure |
| `task_resumed` | `task_id` | Resume accepted; the task rechecks its screen before more input |
| `recognition_evidence` | `task_id`, `stage`, `confidence`, `bbox` | Confirmed visual evidence |
| `log` | `level`, `source`, `message`, `data` | Structured UI log |
| `metrics` | counters | Runtime statistics |
| `debug_frame` | `path`, dimensions, `state` | Safe local frame metadata |
| `task_finished` | `reason`, `stats` | Final task result |
| `error` | `code`, `message`, `recoverable`, `details` | Structured failure |
| `login_stage` | `stage` | Login task progress |
| `settings_reloaded` | — | Configuration reload completed |
| `window_diagnostics` | diagnostic details | Window discovery results |
| `template_match` | match details | Explicit template test result |
| `window_health` | health details | Foreground and capture recovery status |

Unknown message types may be logged and ignored for forward compatibility. A protocol version mismatch is an error.

## Queue and recovery behavior

When a Tauri `start_task` or `start_tasks` command is issued while the automation service is stopped, the Rust supervisor starts the service before forwarding the command. A direct command written to the Python process still requires that process to be running.

The queue preserves the order supplied in `start_tasks`. A task that completes normally starts the next queued task. `stop_task` clears both the active task and every remaining queued task.

`task_finished` with reason `window_recovery_timeout` means the 300-second foreground/capture recovery budget expired. The task and pending queue are cleared, and the engine remains available. Recovery entry and success are logged once, allowing manual refocus during the retry period.

Other task timeouts, engine task errors, and a nonrecoverable error event pause the active task instead of silently advancing the queue. The active task and remaining entries are retained. `resume_task` starts the service again if it crashed, then re-enters the active task's screen router; it does not resume blind input from the old stage. F3 toggles engine startup/shutdown outside the IPC transport. Shutdown releases input and clears the task queue; it emits `task_finished` with `user_stopped`. Shortcut lifecycle logs use source `engine.lifecycle` and carry `engine_status` so the interface updates immediately.

The frontend saves UI events through the Tauri `append_ui_log` command. Engine and UI events are appended to `%LOCALAPPDATA%\IdentityV-Assistant\logs\YYYY-MM-DD.jsonl`.
