from typing import Annotated, Any, Literal

from pydantic import BaseModel, ConfigDict, Field, StrictBool, StrictInt, StrictStr, ValidationInfo, field_validator


class _ConfigModel(BaseModel):
    model_config = ConfigDict(frozen=True)


NonEmptyString = Annotated[StrictStr, Field(min_length=1)]
PositiveFps = Annotated[StrictInt, Field(ge=1, le=120)]
PositiveInt = Annotated[StrictInt, Field(ge=1)]
UnitInterval = Annotated[float, Field(ge=0.0, le=1.0)]
HeartbeatInterval = Annotated[float, Field(ge=0.1, le=60.0)]
TaskTimeout = Annotated[StrictInt, Field(ge=30, le=900)]
DelayMilliseconds = Annotated[StrictInt, Field(ge=0, le=10_000)]
PositionJitter = Annotated[StrictInt, Field(ge=1, le=50)]
TimeJitter = Annotated[StrictInt, Field(ge=1, le=2_000)]


class WindowConfig(_ConfigModel):
    process_names: tuple[NonEmptyString, ...] = ("dwrg.exe", "IdentityV.exe")
    title_contains: tuple[NonEmptyString, ...] = ("第五人格", "Identity V")
    class_names: tuple[NonEmptyString, ...] = ()


class CaptureConfig(_ConfigModel):
    backend: Literal["auto", "wgc", "dxcam", "mss"] = "wgc"
    idle_fps: PositiveFps = 3
    active_fps: PositiveFps = 10


class VisionConfig(_ConfigModel):
    template_threshold: UnitInterval = 0.85
    state_confirmation_count: PositiveInt = 2

    @field_validator("template_threshold", mode="before")
    @classmethod
    def reject_non_numeric_threshold(cls, value: Any) -> Any:
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise ValueError("template_threshold must be a number")
        return value


class AutomationConfig(_ConfigModel):
    login_platform: Literal["android", "ios"] = "android"
    login_timeout_seconds: TaskTimeout = 300
    screenshot_after_click_ms: DelayMilliseconds = 500
    click_after_detection_ms: DelayMilliseconds = 500
    click_position_jitter_px: PositionJitter = 6
    click_interval_jitter_ms: TimeJitter = 120
    click_duration_jitter_ms: TimeJitter = 15
    blackjack_infinite_loop: StrictBool = False
    hunter_match_infinite_loop: StrictBool = False
    hunter_rank_infinite_loop: StrictBool = False
    hide_seek_infinite_loop: StrictBool = False
    camera_turn_scale_percent: Annotated[StrictInt, Field(ge=20, le=300)] = 100


class EngineConfig(_ConfigModel):
    memory_protection: StrictBool = False
    heartbeat_interval_seconds: HeartbeatInterval = 2.0
    window: WindowConfig = Field(default_factory=WindowConfig)
    capture: CaptureConfig = Field(default_factory=CaptureConfig)
    vision: VisionConfig = Field(default_factory=VisionConfig)
    automation: AutomationConfig = Field(default_factory=AutomationConfig)
    debug: dict[str, Any] = Field(default_factory=dict)

    @field_validator("heartbeat_interval_seconds", mode="before")
    @classmethod
    def reject_non_numeric_intervals(cls, value: Any) -> Any:
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise ValueError("interval must be a number")
        return value

    @field_validator("debug", mode="before")
    @classmethod
    def require_debug_object(cls, value: Any) -> Any:
        if not isinstance(value, dict):
            raise ValueError("debug must be an object")
        return value
