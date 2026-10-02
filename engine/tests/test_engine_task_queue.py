import asyncio

import pytest

from app.engine import AutomationEngine
from config.schema import EngineConfig
from core.enums import TaskFinishReason


class Writer:
    def __init__(self):
        self.events = []
        self.paused = asyncio.Event()

    async def send(self, message):
        self.events.append(message)
        if message["type"] == "task_paused":
            self.paused.set()


@pytest.mark.parametrize("failure", [
    TaskFinishReason.TIMEOUT, TaskFinishReason.WINDOW_LOST,
    TaskFinishReason.ENGINE_ERROR, "exception", "event", "release",
])
def test_error_pauses_preserves_queue_and_resume_reroutes(failure):
    async def exercise():
        engine = AutomationEngine(EngineConfig())
        writer = Writer()
        engine.writer = writer
        released = []
        def release_all():
            released.append(True)
            if failure == "release" and len(released) == 1:
                raise RuntimeError("key release failed")

        engine.context.input.release_all = release_all
        runs = []

        class First:
            def __init__(self):
                self.cycles = 0

            async def run(self, ctx, control):
                runs.append(("first", self.cycles))
                if len(runs) == 1:
                    self.cycles = 3
                    if failure == "exception":
                        raise RuntimeError("input failed")
                    if failure == "event":
                        await ctx.emit("error", recoverable=False, message="input failed")
                    if failure == "release":
                        return TaskFinishReason.COMPLETED
                    return failure
                return TaskFinishReason.COMPLETED

        class Second:
            async def run(self, ctx, control):
                runs.append(("second", 0))
                return TaskFinishReason.COMPLETED

        engine._task_factories = lambda: {"first": First, "second": Second}
        await engine._handle_command({"type": "start_tasks", "tasks": ["first", "second"]})
        await asyncio.wait_for(writer.paused.wait(), 1)
        assert engine.task_status == "paused"
        assert engine._active_task_id == "first"
        assert engine._task_queue == ["second"]
        assert runs == [("first", 0)]
        assert released
        assert not any(e["type"] == "task_finished" for e in writer.events)
        await engine._handle_command({"type": "resume_task"})
        await engine._task_runner
        await engine._task_runner
        assert runs == [("first", 0), ("first", 3), ("second", 0)]
        assert engine.task_status == "idle"
        assert engine._task_queue == []

    asyncio.run(exercise())


def test_stop_paused_error_does_not_start_next_task():
    async def exercise():
        engine = AutomationEngine(EngineConfig())
        engine.writer = Writer()

        class Failing:
            async def run(self, ctx, control):
                return TaskFinishReason.WINDOW_LOST

        engine._task_factories = lambda: {"first": Failing, "second": Failing}
        await engine._handle_command({"type": "start_tasks", "tasks": ["first", "second"]})
        await asyncio.wait_for(engine.writer.paused.wait(), 1)
        await engine._handle_command({"type": "stop_task"})
        assert engine._task_queue == []
        assert engine._active_task_id is None
        assert engine._task_control is None
        assert engine.task_status == "idle"
        assert [e.get("task_id") for e in engine.writer.events if e["type"] == "task_started"] == ["first"]
        assert engine.writer.events[-1]["reason"] == "user_stopped"

    asyncio.run(exercise())


def test_pause_command_reports_release_failure_without_crashing_engine():
    from tasks.base import TaskControl

    async def exercise():
        engine = AutomationEngine(EngineConfig())
        engine.writer = Writer()
        engine._task_control = TaskControl()
        engine._active_task_id = "blackjack"
        engine._task_queue = ["hunter_match"]

        def fail():
            raise RuntimeError("release failed")

        engine.context.input.release_all = fail
        await engine._handle_command({"type": "pause_task"})
        assert engine.task_status == "paused"
        assert engine._task_queue == ["hunter_match"]
        assert engine.writer.events[-1]["type"] == "task_paused"
        assert engine.writer.events[-1]["reason"] == "input_failed"

    asyncio.run(exercise())
