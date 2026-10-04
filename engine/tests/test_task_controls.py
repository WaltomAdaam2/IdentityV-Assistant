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


def test_resume_during_click_route_discards_old_target():
    from types import SimpleNamespace
    from tasks.blackjack_task import BlackjackTask
    from config.schema import EngineConfig
    from core.models import Rect

    async def exercise():
        task = BlackjackTask()
        clicks = []

        async def noop(*_):
            pass

        async def capture(*_):
            return object()

        async def confirmed(*_):
            return True

        async def window(*_):
            return SimpleNamespace(client_rect=Rect(0, 0, 1920, 1080))

        async def paused_route(*_):
            task._resume_recheck = True

        task._sleep, task._capture, task._confirmed = noop, capture, confirmed
        task._input_window, task._curved_move = window, paused_route
        ctx = SimpleNamespace(config=EngineConfig(), input=SimpleNamespace(
            _jitter_point=lambda point, bounds=None: point, mouse_down=lambda _: clicks.append(True),
        ))
        assert not await task._click(ctx, TaskControl(), "home", "open_book")
        assert not clicks

    asyncio.run(exercise())
