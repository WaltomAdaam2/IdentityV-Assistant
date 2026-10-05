import asyncio
from types import SimpleNamespace

import pytest

from app.engine import AutomationEngine
from config.schema import EngineConfig
from core.enums import EngineStatus
from tasks.base import TaskControl


class Writer:
    def __init__(self):
        self.events = []

    async def send(self, message):
        self.events.append(message)


@pytest.mark.parametrize("paused", [False, True])
@pytest.mark.parametrize("action", [
    {"type": "press", "key": "w"},
    {"type": "click", "x": 960, "y": 540},
])
def test_developer_input_cannot_interfere_with_active_task(paused, action):
    async def exercise():
        engine = AutomationEngine(EngineConfig())
        engine.writer = Writer()
        engine._developer_input_enabled = True
        engine._task_control = TaskControl()
        if paused:
            engine._task_control.pause()
        engine._task_runner = asyncio.create_task(asyncio.Event().wait())
        calls = []
        engine.context.input = SimpleNamespace(
            press=lambda *_: calls.append("press"),
            click=lambda *_: calls.append("click"),
            release_all=lambda: calls.append("release"),
        )
        engine.context.window = SimpleNamespace(find_window=lambda: calls.append("window"))
        try:
            await engine._handle_command({"type": "input_test", "action": action, "request_id": "input-1"})
            assert not calls
            error = engine.writer.events[-1]
            assert error["type"] == "error"
            assert error["code"] == "INPUT_FAILED"
            assert error["request_id"] == "input-1"
            assert "Stop the active task" in error["message"]
        finally:
            engine._task_runner.cancel()
            await asyncio.gather(engine._task_runner, return_exceptions=True)

    asyncio.run(exercise())


@pytest.mark.parametrize("error_type", [OSError, Exception])
@pytest.mark.parametrize("release_fails", [False, True])
def test_developer_input_failure_is_reported_and_engine_stays_available(error_type, release_fails):
    async def exercise():
        engine = AutomationEngine(EngineConfig())
        engine.writer = Writer()
        engine.status = EngineStatus.IDLE
        engine._developer_input_enabled = True
        releases = []

        def fail(*_):
            raise error_type("input injection failed")

        def release():
            releases.append(True)
            if release_fails:
                raise OSError("input cleanup failed")

        engine.context.input = SimpleNamespace(press=fail, release_all=release)
        engine.context.window = SimpleNamespace(find_window=lambda: None)
        await engine._handle_command({
            "type": "input_test", "action": {"type": "press", "key": "w"}, "request_id": "input-2",
        })
        error = engine.writer.events[0]
        assert error["type"] == "error"
        assert error["code"] == "INPUT_FAILED"
        assert error["request_id"] == "input-2"
        assert error["message"] == "input injection failed"
        assert releases == [True]
        if release_fails:
            assert engine.writer.events[-1]["code"] == "INPUT_FAILED"
            assert "input cleanup failed" in engine.writer.events[-1]["message"]
        await engine._handle_command({"type": "ping"})
        assert engine.writer.events[-1]["type"] == "heartbeat"
        assert engine.status == EngineStatus.IDLE
        assert not engine._shutdown.is_set()

    asyncio.run(exercise())
