from __future__ import annotations

import asyncio
from types import SimpleNamespace

import pytest

from app.engine import AutomationEngine
from config.schema import EngineConfig, WindowConfig
from core.enums import TaskFinishReason
from core.models import Rect
from window.base import WindowInfo
from window.windows import WindowsWindowManager


def test_window_manager_delegates_current_options_and_restoration():
    calls = []
    manager = object.__new__(WindowsWindowManager)
    manager.config = WindowConfig()
    manager.current_hwnd = lambda: 42
    manager._style_controller = SimpleNamespace(
        apply=lambda *args: calls.append(args) or True,
        restore=lambda: calls.append('restore') or True,
    )
    assert manager.apply_automation_style()
    manager.config = WindowConfig(resolution='1280x720', position='bottom_left')
    assert manager.apply_automation_style()
    assert manager.restore_automation_style()
    assert calls == [(42, '1600x900', 'top_right'), (42, '1280x720', 'bottom_left'), 'restore']
    manager.current_hwnd = lambda: None
    assert not manager.apply_automation_style()
    assert len(calls) == 3


@pytest.mark.parametrize("start_task", [False, True])
def test_engine_only_applies_window_style_for_automation_tasks(start_task: bool) -> None:
    heartbeat = asyncio.Event()
    task_started = asyncio.Event()

    class Writer:
        async def send(self, message: dict) -> None:
            if message["type"] == "heartbeat":
                heartbeat.set()

    class Reader:
        async def messages(self):
            await heartbeat.wait()
            assert window.applied == 0
            if start_task:
                yield {"type": "start_task", "task": "login_lobby"}
                await task_started.wait()
            yield {"type": "shutdown"}

    class Task:
        async def run(self, ctx, control):
            assert window.applied >= 1
            task_started.set()
            await control.cancelled.wait()
            return TaskFinishReason.USER_STOPPED

    class Window:
        applied = 0
        restored = 0

        def find_window(self) -> WindowInfo:
            return WindowInfo(42, "Game", "Class", 1, Rect(0, 0, 1920, 1080), 1.0)

        def apply_automation_style(self) -> bool:
            self.applied += 1
            return True

        def restore_automation_style(self) -> bool:
            self.restored += 1
            return True

    engine = AutomationEngine(EngineConfig())
    window = Window()
    engine.writer = Writer()
    engine.reader = Reader()
    engine.context.window = window
    engine.context.input = SimpleNamespace(release_all=lambda: None)
    engine._task_factories = lambda: {"login_lobby": Task}

    async def run():
        await asyncio.wait_for(engine.run(), timeout=1)

    asyncio.run(run())

    assert (window.applied > 0) == start_task
    assert window.restored == 1
