from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class WindowConfig:
    process_names: tuple[str, ...] = ("dwrg.exe", "IdentityV.exe")
    title_contains: tuple[str, ...] = ("第五人格", "Identity V")
    class_names: tuple[str, ...] = ()


@dataclass(frozen=True)
class CaptureConfig:
    backend: str = "auto"
    idle_fps: int = 3
    active_fps: int = 10


@dataclass(frozen=True)
class VisionConfig:
    template_threshold: float = 0.85
    state_confirmation_count: int = 2


@dataclass(frozen=True)
class EngineConfig:
    dry_run: bool = True
    heartbeat_interval_seconds: float = 2.0
    mock_step_delay_seconds: float = 1.0
    window: WindowConfig = field(default_factory=WindowConfig)
    capture: CaptureConfig = field(default_factory=CaptureConfig)
    vision: VisionConfig = field(default_factory=VisionConfig)
    debug: dict[str, Any] = field(default_factory=dict)
