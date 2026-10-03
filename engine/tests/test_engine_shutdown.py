from __future__ import annotations

import asyncio
from types import SimpleNamespace

import pytest

from app.engine import AutomationEngine
from config.schema import EngineConfig
from core.enums import EngineStatus
from core.models import Rect
from tasks.base import TaskControl
from window.base import WindowInfo


class Window:
    def __init__(self) -> None:
        self.applied = 0
        self.restored = 0

    def find_window(self) -> WindowInfo:
        return WindowInfo(42, "Game", "Class", 1, Rect(0, 0, 1920, 1080), 1.0)

    def apply_automation_style(self) -> bool:
        self.applied += 1
        return True

    def restore_automation_style(self) -> bool:
        self.restored += 1
        return True


class Writer:
    async def send(self, message: dict) -> None:
        pass


class Reader:
    async def messages(self):
        yield {"type": "shutdown"}


def make_engine() -> AutomationEngine:
    engine = AutomationEngine(EngineConfig())
    engine.context.window = Window()
    engine.context.input = SimpleNamespace(release_all=lambda: None)
    engine.writer = Writer()
    engine.reader = Reader()
    return engine


def test_shutdown_cancels_waiting_task_before_restoring_window() -> None:
    async def run() -> None:
        engine = make_engine()
        task_started = asyncio.Event()
        task_cancelled = asyncio.Event()

        async def waiting_task() -> None:
            task_started.set()
            try:
                await asyncio.Event().wait()
            finally:
                task_cancelled.set()

        engine._task_control = TaskControl()
        engine._task_runner = asyncio.create_task(waiting_task())
        await task_started.wait()
        await asyncio.wait_for(engine.run(), timeout=1)

        assert task_cancelled.is_set()
        assert engine._task_control.cancelled.is_set()
        assert engine.context.window.restored == 1
        assert engine.status == EngineStatus.STOPPED

    asyncio.run(run())


@pytest.mark.parametrize("failure", ["ready", "task_stop", "input_release"])
def test_shutdown_restores_window_when_other_cleanup_fails(failure: str) -> None:
    engine = make_engine()

    async def fail(*args, **kwargs) -> None:
        raise RuntimeError(failure)

    if failure == "ready":
        engine.writer.send = fail
    elif failure == "task_stop":
        engine._stop_task = fail
    else:
        engine._release_inputs = fail

    with pytest.raises(RuntimeError, match=failure):
        asyncio.run(engine.run())

    assert engine.context.window.applied == 1
    assert engine.context.window.restored == 1
    assert engine.status == EngineStatus.STOPPED


def test_shutdown_retries_transient_window_restore_failure() -> None:
    engine = make_engine()
    attempts = 0

    def restore() -> bool:
        nonlocal attempts
        attempts += 1
        if attempts == 1:
            raise OSError("temporary window style failure")
        return True

    engine.context.window.restore_automation_style = restore
    asyncio.run(engine.run())

    assert attempts == 2
    assert engine.status == EngineStatus.STOPPED
