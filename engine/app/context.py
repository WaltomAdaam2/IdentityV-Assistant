from dataclasses import dataclass
from typing import Awaitable, Callable

from capture.manager import CaptureManager
from config.schema import EngineConfig
from input.controller import InputController
from runtime.cancellation import CancellationToken
from runtime.metrics import RuntimeMetrics
from states.history import StateHistory
from window.base import WindowManager

Emitter = Callable[[str], Awaitable[None]]


@dataclass
class AutomationContext:
    config: EngineConfig
    window: WindowManager
    capture: CaptureManager
    input: InputController
    metrics: RuntimeMetrics
    cancellation: CancellationToken
    state_history: StateHistory
    emit: Emitter
