import asyncio
from types import SimpleNamespace

import pytest

import tasks.blackjack_task as module
from tasks.blackjack_task import BlackjackTask, _WindowLost


def test_unresponsive_window_is_still_usable() -> None:
    task = BlackjackTask()
    window = SimpleNamespace(is_responding=False, is_minimized=False, is_foreground=True)
    context = SimpleNamespace(window=SimpleNamespace(find_window=lambda: window))
    assert task._window(context) is window


def test_capture_retries_for_300_seconds_and_recovers(monkeypatch) -> None:
    async def scenario(recover_after: float | None):
        task = BlackjackTask()
        elapsed = [0.0]
        events = []

        async def check(_ctx, _control):
            pass

        async def wait(_ctx, _control, seconds):
            elapsed[0] += seconds

        async def log(_ctx, level, message):
            events.append((level, message))

        def capture(_ctx):
            if recover_after is None or elapsed[0] < recover_after:
                raise _WindowLost("temporary capture failure")
            return "frame"

        monkeypatch.setattr(module, "monotonic", lambda: elapsed[0])
        task._check = check
        task._sleep = wait
        task._log = log
        task._capture_once = capture
        if recover_after is None:
            with pytest.raises(_WindowLost, match="300 seconds"):
                await task._capture(None, None)
            assert elapsed[0] == 300
            assert [level for level, _ in events] == ["warning", "error"]
        else:
            assert await task._capture(None, None) == "frame"
            assert elapsed[0] == recover_after
            assert [level for level, _ in events] == ["warning", "info"]

    asyncio.run(scenario(299.75))
    asyncio.run(scenario(None))
