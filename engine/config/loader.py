import json
from pathlib import Path
from typing import Any

from config.schema import EngineConfig


def _mapping(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be an object")
    return value


def parse_config(data: dict[str, Any]) -> EngineConfig:
    root = _mapping(data, "config")
    window_data = _mapping(root.get("game_window", {}), "game_window")
    capture_data = _mapping(root.get("capture", {}), "capture")
    vision_data = _mapping(root.get("vision", {}), "vision")
    engine_data = _mapping(root.get("engine", {}), "engine")
    return EngineConfig.model_validate(
        {
            "dry_run": engine_data.get("dry_run", True),
            "memory_protection": engine_data.get("memory_protection", False),
            "heartbeat_interval_seconds": engine_data.get("heartbeat_interval_seconds", 2.0),
            "mock_step_delay_seconds": engine_data.get("mock_step_delay_seconds", 1.0),
            "window": {
                "process_names": window_data.get("process_names", ["dwrg.exe", "IdentityV.exe"]),
                "title_contains": window_data.get("title_contains", ["第五人格", "Identity V"]),
                "class_names": window_data.get("class_names", []),
                "background_mode": window_data.get("background_mode", True),
                "resolution": window_data.get("resolution", "1600x900"),
                "position": window_data.get("position", "free"),
                "restore_after_task": window_data.get("restore_after_task", True),
            },
            "capture": {
                "backend": capture_data.get("backend", "wgc"),
                "idle_fps": capture_data.get("idle_fps", 3),
                "active_fps": capture_data.get("active_fps", 10),
            },
            "vision": {
                "template_threshold": vision_data.get("template_threshold", 0.85),
                "state_confirmation_count": vision_data.get("state_confirmation_count", 2),
            },
            "debug": _mapping(root.get("debug", {}), "debug"),
        },
        context={"from_file": True},
    )


def load_config(path: Path) -> EngineConfig:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise ValueError(f"unable to read config: {error}") from error
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid JSON: {error.msg}") from error
    return parse_config(_mapping(raw, "config"))
