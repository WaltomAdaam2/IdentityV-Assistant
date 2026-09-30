# IPC protocol

Protocol version: `1`. Transport: one UTF-8 JSON object per line over Python stdin/stdout. Python stdout is reserved for protocol messages; raw diagnostics use stderr.

Every message includes `version`, `type`, and an ISO-8601 `timestamp`. Commands also include a `request_id`; the Rust bridge supplies missing envelope fields.

## Rust to Python

| Type | Main fields | Purpose |
| --- | --- | --- |
| `start_task` | `task` | Start one registered task |
| `start_tasks` | ordered `tasks` array | Run enabled tasks in list order |
| `pause_task` | — | Pause the active task |
| `resume_task` | — | Resume it |
| `stop_task` | — | Cancel it and release held inputs |
| `ping` | — | Request a liveness response |
| `shutdown` | — | Graceful engine shutdown |
| `detect_window` | — | Run window diagnostics |
| `capture_frame` | — | Save one debug frame |
| `test_template` | `template` | Match one registered template |
| `input_test` | `action` | Developer input test; first requires an explicit `set_enabled` action |

## Python to Rust

| Type | Main fields | Purpose |
| --- | --- | --- |
| `ready` | `engine_version` | Engine initialized |
| `heartbeat` | `engine_status`, `task_status` | Liveness and lifecycle |
| `task_state` | `state`, `previous_state`, `confidence` | Stable detected state |
| `state_transition` | `from`, `to`, `confidence`, `duration_in_previous_state` | One event per transition |
| `planned_action` | `action`, `input`, `target` | Action sent to the game |
| `task_started` | `task_id` | Task selected from the queue |
| `task_progress` | `task_id`, `stage`, `role`, `completed_cycles`, `infinite_loop` | Current Blackjack progress |
| `recognition_evidence` | `task_id`, `stage`, `confidence`, `bbox` | Confirmed visual evidence |
| `log` | `level`, `source`, `message`, `data` | Structured UI log |
| `metrics` | counters | Runtime statistics |
| `debug_frame` | `path`, dimensions, `state` | Safe local frame metadata |
| `task_finished` | `reason`, `stats` | Final task result |
| `error` | `code`, `message`, `recoverable`, `details` | Structured failure |

Unknown message types may be logged and ignored for forward compatibility. A protocol version mismatch is an error.

The frontend saves UI events through the Tauri `append_ui_log` command. Engine and UI events are appended to `%LOCALAPPDATA%\IdentityV-Assistant\logs\YYYY-MM-DD.jsonl`.
