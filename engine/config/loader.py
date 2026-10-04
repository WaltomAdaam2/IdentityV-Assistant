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
    automation_data = _mapping(root.get("automation", {}), "automation")
    return EngineConfig.model_validate(
        {
            "memory_protection": engine_data.get("memory_protection", False),
            "heartbeat_interval_seconds": engine_data.get("heartbeat_interval_seconds", 2.0),
            "window": {
                "resolution": window_data.get("resolution", "1600x900"),
                "position": window_data.get("position", "top_right"),
                "process_names": window_data.get("process_names", ["dwrg.exe", "IdentityV.exe"]),
                "title_contains": window_data.get("title_contains", ["第五人格", "Identity V"]),
                "class_names": window_data.get("class_names", []),
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
            "automation": {
                "login_platform": automation_data.get("login_platform", "android"),
                "login_timeout_seconds": automation_data.get("login_timeout_seconds", 300),
                "screenshot_after_click_ms": automation_data.get("screenshot_after_click_ms", 500),
                "click_after_detection_ms": automation_data.get("click_after_detection_ms", 500),
                "click_position_jitter_px": automation_data.get("click_position_jitter_px", 6),
                "click_interval_jitter_ms": automation_data.get("click_interval_jitter_ms", 120),
                "click_duration_jitter_ms": automation_data.get("click_duration_jitter_ms", 15),
                "blackjack_infinite_loop": automation_data.get("blackjack_infinite_loop", False),
                "hunter_match_infinite_loop": automation_data.get("hunter_match_infinite_loop", False),
                "hunter_rank_infinite_loop": automation_data.get("hunter_rank_infinite_loop", False),
                "hide_seek_infinite_loop": automation_data.get("hide_seek_infinite_loop", False),
                "camera_turn_scale_percent": automation_data.get("camera_turn_scale_percent", 100),
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
