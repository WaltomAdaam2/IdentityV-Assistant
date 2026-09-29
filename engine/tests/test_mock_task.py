import asyncio

from app.context import AutomationContext
from capture.manager import CaptureManager
from config.schema import EngineConfig
from core.enums import TaskFinishReason
from input.base import InputBackend
from input.controller import InputController
from runtime.cancellation import CancellationToken
from runtime.metrics import RuntimeMetrics
from states.history import StateHistory
from tasks.base import TaskControl
from tasks.mock_task import MockTask
from window.windows import WindowsWindowManager


class NoopInput(InputBackend):
    def move_mouse(self, point): pass
    def mouse_down(self, button="left"): pass
    def mouse_up(self, button="left"): pass
    def key_down(self, key): pass
    def key_up(self, key): pass


def test_mock_task_emits_states_and_finishes() -> None:
    async def exercise() -> None:
        messages: list[tuple[str, dict]] = []

        async def emit(event_type: str, **payload: object) -> None:
            messages.append((event_type, payload))

        config = EngineConfig(mock_step_delay_seconds=0.001)
        context = AutomationContext(config, WindowsWindowManager(config.window), CaptureManager(config.capture), InputController(NoopInput()), RuntimeMetrics(), CancellationToken(), StateHistory(config.vision.state_confirmation_count), emit)
        result = await MockTask().run(context, TaskControl())
        assert result == TaskFinishReason.COMPLETED
        assert len([message for message in messages if message[0] == "task_state"]) == 7
        assert len([message for message in messages if message[0] == "state_transition"]) == 7

    asyncio.run(exercise())
