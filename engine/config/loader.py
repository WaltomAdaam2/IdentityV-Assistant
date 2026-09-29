import json
from pathlib import Path
from typing import Any

from config.schema import CaptureConfig, EngineConfig, VisionConfig, WindowConfig


def _mapping(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be an object")
    return value


def _strings(value: Any, name: str) -> tuple[str, ...]:
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise ValueError(f"{name} must be a list of non-empty strings")
    return tuple(value)


def _number(value: Any, name: str, minimum: float, maximum: float) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not minimum <= value <= maximum:
        raise ValueError(f"{name} must be between {minimum} and {maximum}")
    return float(value)


def _positive_int(value: Any, name: str, maximum: int = 120) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 1 <= value <= maximum:
        raise ValueError(f"{name} must be an integer between 1 and {maximum}")
    return value


def parse_config(data: dict[str, Any]) -> EngineConfig:
    root = _mapping(data, "config")
    window_data = _mapping(root.get("game_window", {}), "game_window")
    capture_data = _mapping(root.get("capture", {}), "capture")
    vision_data = _mapping(root.get("vision", {}), "vision")
    engine_data = _mapping(root.get("engine", {}), "engine")
    process_names = _strings(window_data.get("process_names", ["dwrg.exe", "IdentityV.exe"]), "game_window.process_names")
    titles = _strings(window_data.get("title_contains", ["第五人格", "Identity V"]), "game_window.title_contains")
    classes = _strings(window_data.get("class_names", []), "game_window.class_names") if window_data.get("class_names", []) else ()
    backend = capture_data.get("backend", "auto")
    if backend not in {"auto", "dxcam", "mss"}:
        raise ValueError("capture.backend must be auto, dxcam, or mss")
    threshold = _number(vision_data.get("template_threshold", 0.85), "vision.template_threshold", 0.0, 1.0)
    confirmation = vision_data.get("state_confirmation_count", 2)
    if isinstance(confirmation, bool) or not isinstance(confirmation, int) or confirmation < 1:
        raise ValueError("vision.state_confirmation_count must be a positive integer")
    heartbeat = _number(engine_data.get("heartbeat_interval_seconds", 2.0), "engine.heartbeat_interval_seconds", 0.1, 60.0)
    mock_delay = _number(engine_data.get("mock_step_delay_seconds", 1.0), "engine.mock_step_delay_seconds", 0.05, 60.0)
    dry_run = engine_data.get("dry_run", True)
    if not isinstance(dry_run, bool):
        raise ValueError("engine.dry_run must be a boolean")
    return EngineConfig(dry_run=dry_run, heartbeat_interval_seconds=heartbeat, mock_step_delay_seconds=mock_delay, window=WindowConfig(process_names, titles, classes), capture=CaptureConfig(backend, _positive_int(capture_data.get("idle_fps", 3), "capture.idle_fps"), _positive_int(capture_data.get("active_fps", 10), "capture.active_fps")), vision=VisionConfig(threshold, confirmation), debug=_mapping(root.get("debug", {}), "debug"))


def load_config(path: Path) -> EngineConfig:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise ValueError(f"unable to read config: {error}") from error
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid JSON: {error.msg}") from error
    return parse_config(_mapping(raw, "config"))
