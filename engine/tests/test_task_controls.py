import asyncio

from tasks.base import TaskControl


def test_task_control_pause_resume_and_cancel() -> None:
    async def exercise() -> None:
        control = TaskControl()
        control.pause()
        blocked = asyncio.create_task(control.wait_if_paused())
        await asyncio.sleep(0)
        assert not blocked.done()
        control.resume()
        await blocked
        control.cancel()
        assert control.cancelled.is_set()
        assert control.resumed.is_set()

    asyncio.run(exercise())


def test_pause_duration_freezes_game_timeout_clock(monkeypatch):
    import tasks.base as module
    from tasks.blackjack_task import BlackjackTask

    elapsed = [100.0]
    monkeypatch.setattr(module, "monotonic", lambda: elapsed[0])
    monkeypatch.setattr("tasks.blackjack_task.monotonic", lambda: elapsed[0])
    control = TaskControl()
    task = BlackjackTask()
    control.pause()
    elapsed[0] += 600
    control.pause()
    assert task._now(control) == 100
    control.resume()
    elapsed[0] += 5
    assert task._now(control) == 105
