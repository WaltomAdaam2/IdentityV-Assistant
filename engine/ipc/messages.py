from __future__ import annotations

from datetime import UTC, datetime
from typing import Any

PROTOCOL_VERSION = 1
ENGINE_VERSION = "0.1.10"


def timestamp() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def event(event_type: str, **payload: Any) -> dict[str, Any]:
    return {"version": PROTOCOL_VERSION, "type": event_type, "timestamp": timestamp(), **payload}


COMMAND_TYPES = frozenset({
    "start_task", "start_tasks", "pause_task", "resume_task", "stop_task", "shutdown", "ping",
    "detect_window", "capture_frame", "test_template", "input_test", "reload_settings", "enable_window_layout",
    "task_preflight",
})
