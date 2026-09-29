from enum import Enum


class EngineStatus(str, Enum):
    STOPPED = "stopped"
    STARTING = "starting"
    IDLE = "idle"
    RUNNING = "running"
    PAUSED = "paused"
    STOPPING = "stopping"
    ERROR = "error"


class TaskFinishReason(str, Enum):
    COMPLETED = "completed"
    WEEKLY_LIMIT = "weekly_limit"
    USER_STOPPED = "user_stopped"
    TIMEOUT = "timeout"
    ENGINE_ERROR = "engine_error"
    WINDOW_LOST = "window_lost"


class ErrorCode(str, Enum):
    ENGINE_START_FAILED = "ENGINE_START_FAILED"
    ENGINE_CRASHED = "ENGINE_CRASHED"
    IPC_PARSE_ERROR = "IPC_PARSE_ERROR"
    WINDOW_NOT_FOUND = "WINDOW_NOT_FOUND"
    WINDOW_INVALID = "WINDOW_INVALID"
    CAPTURE_FAILED = "CAPTURE_FAILED"
    INPUT_FAILED = "INPUT_FAILED"
    TEMPLATE_NOT_FOUND = "TEMPLATE_NOT_FOUND"
    OCR_FAILED = "OCR_FAILED"
    STATE_UNKNOWN_TIMEOUT = "STATE_UNKNOWN_TIMEOUT"
    TASK_CANCELLED = "TASK_CANCELLED"
    CONFIG_INVALID = "CONFIG_INVALID"
