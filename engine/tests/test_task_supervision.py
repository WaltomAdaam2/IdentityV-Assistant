import asyncio
from types import SimpleNamespace

import pytest

from tasks.base import TaskControl
from tasks.blackjack_task import BlackjackTask
from tasks.hunter_match_task import HunterMatchTask, HunterRankTask


def context():
    releases = []
    return SimpleNamespace(input=SimpleNamespace(release_all=lambda: releases.append(True))), releases


@pytest.mark.parametrize("task_class", [BlackjackTask, HunterMatchTask, HunterRankTask])
@pytest.mark.parametrize("waiting_in", ["sleep", "capture"])
def test_worker_failure_interrupts_observation_immediately(task_class, waiting_in):
    async def exercise():
        task = task_class()
        task.role = "survivor"
        ctx, releases = context()
        observing = asyncio.Event()
        movement_stopped = asyncio.Event()

        async def blocked(*_):
            observing.set()
            await asyncio.Event().wait()

        async def sleep(*_):
            if waiting_in == "sleep":
                await blocked()
            else:
                await asyncio.sleep(0)

        async def movement(*_):
            try:
                await asyncio.Event().wait()
            finally:
                movement_stopped.set()

        async def actions(*_):
            await observing.wait()
            raise OSError("worker injection failed")

        task._sleep = sleep
        task._capture = blocked
        task._movement = movement
        task._survivor = task._combat = actions
        with pytest.raises(OSError, match="worker injection failed"):
            await asyncio.wait_for(task._play(ctx, TaskControl()), 0.5)
        assert movement_stopped.is_set()
        assert releases

    asyncio.run(exercise())


@pytest.mark.parametrize("task_class", [BlackjackTask, HunterMatchTask, HunterRankTask])
def test_result_transition_reports_worker_cleanup_failure(task_class):
    async def exercise():
        task = task_class()
        task.role = "survivor"
        ctx, releases = context()

        async def movement(*_):
            try:
                await asyncio.Event().wait()
            finally:
                raise OSError("movement release failed")

        async def actions(*_):
            await asyncio.Event().wait()

        async def sleep(*_):
            await asyncio.sleep(0)

        async def capture(*_):
            return object()

        async def confirmed(_ctx, _control, _image, name):
            return name == "return_lobby"

        async def progress(*_):
            pass

        task._movement = movement
        task._survivor = task._combat = actions
        task._sleep = sleep
        task._capture = capture
        task._confirmed = confirmed
        task._progress = progress
        with pytest.raises(OSError, match="movement release failed"):
            await asyncio.wait_for(task._play(ctx, TaskControl()), 0.5)
        assert releases

    asyncio.run(exercise())


@pytest.mark.parametrize("cleanup_fails", [False, True])
def test_blackjack_role_change_restarts_movement_after_cleanup(cleanup_fails):
    async def exercise():
        task = BlackjackTask()
        task.role = "hunter"
        task.detector = SimpleNamespace(role=lambda _: ("survivor", None))
        ctx, releases = context()
        starts = []
        stops = []

        async def movement(*_):
            starts.append(task.role)
            try:
                await asyncio.Event().wait()
            finally:
                stops.append(task.role)
                if cleanup_fails:
                    raise OSError("role cleanup failed")

        async def actions(*_):
            await asyncio.Event().wait()

        async def sleep(*_):
            await asyncio.sleep(0)

        async def capture(*_):
            return object()

        async def confirmed(_ctx, _control, _image, name):
            return name == "return_lobby" and len(starts) == 2

        async def progress(*_):
            pass

        task._movement = movement
        task._hunter = task._survivor = actions
        task._sleep = sleep
        task._capture = capture
        task._confirmed = confirmed
        task._progress = progress
        if cleanup_fails:
            with pytest.raises(OSError, match="role cleanup failed"):
                await asyncio.wait_for(task._play(ctx, TaskControl()), 0.5)
            assert starts == stops == ["hunter"]
        else:
            await asyncio.wait_for(task._play(ctx, TaskControl()), 0.5)
            assert starts == stops == ["hunter", "survivor"]
        assert len(releases) == len(stops)

    asyncio.run(exercise())


def test_surrender_does_not_send_menu_input_after_cleanup_failure(monkeypatch):
    monkeypatch.setattr("tasks.hunter_match_task.SurrenderClock.start", lambda _: SimpleNamespace(due=lambda *_: True))

    async def exercise():
        task = HunterMatchTask()
        ctx, releases = context()
        keys = []

        async def movement(*_):
            try:
                await asyncio.Event().wait()
            finally:
                raise OSError("surrender cleanup failed")

        async def actions(*_):
            await asyncio.Event().wait()

        async def sleep(*_):
            await asyncio.sleep(0)

        async def capture(*_):
            return object()

        async def confirmed(_ctx, _control, _image, name):
            return name == "playing"

        async def key(*args):
            keys.append(args[-1])

        task._movement = movement
        task._combat = actions
        task._sleep = sleep
        task._capture = capture
        task._confirmed = confirmed
        task._displayed_seconds = lambda _: None
        task._key = key
        with pytest.raises(OSError, match="surrender cleanup failed"):
            await asyncio.wait_for(task._play(ctx, TaskControl()), 0.5)
        assert not keys
        assert releases

    asyncio.run(exercise())


def test_nested_hunter_cleanup_failure_survives_cancellation():
    async def exercise():
        task = BlackjackTask()
        started = asyncio.Event()

        async def combo(*_):
            started.set()
            try:
                await asyncio.Event().wait()
            finally:
                raise OSError("combo cleanup failed")

        async def sleep(*_):
            await asyncio.Event().wait()

        task._hunter_combo = combo
        task._sleep = sleep
        runner = asyncio.create_task(task._hunter(None, TaskControl()))
        await started.wait()
        runner.cancel()
        with pytest.raises(OSError, match="combo cleanup failed"):
            await runner

    asyncio.run(exercise())
