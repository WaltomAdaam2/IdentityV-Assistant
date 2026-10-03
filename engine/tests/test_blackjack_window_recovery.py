import asyncio
from types import SimpleNamespace

import pytest

import tasks.blackjack_task as module
from tasks.blackjack_task import BlackjackTask, _WindowLost
from tasks.blackjack_task import _WindowRecoveryTimeout
from tasks.blackjack_task import _Stopped
from config.schema import EngineConfig
from core.enums import TaskFinishReason
from input.windows_sendinput import ForegroundLostError
from tasks.base import TaskControl
from tasks.hide_seek_task import HideSeekTask
from tasks.hunter_match_task import HunterMatchTask, HunterRankTask


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
        released = []
        context = SimpleNamespace(input=SimpleNamespace(release_all=lambda: released.append(True)))
        if recover_after is None:
            with pytest.raises(_WindowRecoveryTimeout, match="300 seconds"):
                await task._capture(context, None)
            assert elapsed[0] == 300
            assert [level for level, _ in events] == ["warning", "error"]
        else:
            assert await task._capture(context, None) == "frame"
            assert elapsed[0] == recover_after
            assert [level for level, _ in events] == ["warning", "info"]
        assert released == [True]

    asyncio.run(scenario(299.75))
    asyncio.run(scenario(None))


@pytest.mark.parametrize("recover_after", [12.5, None])
def test_foreground_recovery_polls_and_allows_manual_intervention(monkeypatch, recover_after):
    async def exercise():
        task = BlackjackTask()
        elapsed = [0.0]
        focus_attempts = []
        released = []
        events = []
        window = SimpleNamespace(is_minimized=False, is_foreground=False)

        def find_window():
            window.is_foreground = recover_after is not None and elapsed[0] >= recover_after
            return window

        context = SimpleNamespace(
            window=SimpleNamespace(
                find_window=find_window,
                bring_to_front=lambda: focus_attempts.append(elapsed[0]),
            ),
            input=SimpleNamespace(release_all=lambda: released.append(True)),
            cancellation=SimpleNamespace(cancelled=False),
        )

        async def wait(_ctx, _control, seconds):
            elapsed[0] += seconds

        async def log(_ctx, level, message):
            events.append((level, message))

        task._now = lambda _: elapsed[0]
        task._sleep = wait
        task._log = log
        task._capture_once = lambda ctx: task._window(ctx)
        if recover_after is None:
            with pytest.raises(_WindowRecoveryTimeout):
                await task._capture(context, TaskControl())
            assert elapsed[0] == 300
            assert [level for level, _ in events] == ["warning", "error"]
        else:
            assert await task._capture(context, TaskControl()) is window
            assert elapsed[0] == recover_after
            assert [level for level, _ in events] == ["warning", "info"]
            assert task._resume_recheck
        assert released == [True]
        assert all(second - first == 0.25 for first, second in zip(focus_attempts, focus_attempts[1:]))

    asyncio.run(exercise())


def test_input_window_recovery_uses_one_deadline_across_capture_attempts():
    async def exercise():
        task = BlackjackTask()
        elapsed = [0.0]
        context = SimpleNamespace(
            input=SimpleNamespace(release_all=lambda: None),
            cancellation=SimpleNamespace(cancelled=False),
        )

        async def wait(_ctx, _control, seconds):
            elapsed[0] += seconds

        async def log(*_):
            pass

        def missing(_ctx):
            raise _WindowLost("lost foreground after capture")

        def intermittent_capture(_ctx):
            if elapsed[0] == 100:
                return "frame"
            raise _WindowLost("capture unavailable")

        task._now = lambda _: elapsed[0]
        task._sleep = wait
        task._log = log
        task._window = missing
        task._capture_once = intermittent_capture
        with pytest.raises(_WindowRecoveryTimeout):
            await task._input_window(context, TaskControl())
        assert elapsed[0] == 300

    asyncio.run(exercise())


@pytest.mark.parametrize("task_type", [BlackjackTask, HunterMatchTask, HunterRankTask, HideSeekTask])
@pytest.mark.parametrize("recovers", [True, False])
def test_foreground_input_race_recovers_and_reroutes_without_counting_failed_cycle(monkeypatch, task_type, recovers):
    async def exercise():
        task = task_type()
        released = []
        events = []
        original_backend = object()
        config = EngineConfig()

        class Backend:
            def __init__(self, hwnd):
                self._expected_hwnd = hwnd

        monkeypatch.setattr(task_type.__module__ + ".WindowsSendInput", Backend)

        async def emit(event, **payload):
            events.append((event, payload))

        context = SimpleNamespace(
            input=SimpleNamespace(backend=original_backend, release_all=lambda: released.append(True)),
            config=config, emit=emit,
        )
        cycle_calls = []

        async def capture(*_):
            if cycle_calls and not recovers:
                raise _WindowRecoveryTimeout("300 seconds")
            return object()

        async def input_window(*_):
            return SimpleNamespace(hwnd=42)

        async def cycle(*_):
            cycle_calls.append(task.cycles)
            if len(cycle_calls) == 1:
                raise ForegroundLostError("foreground changed before SendInput")
            assert task._resume_recheck

        task._capture = capture
        task._input_window = input_window
        task._cycle = cycle
        result = await task.run(context, TaskControl())
        assert result == (TaskFinishReason.COMPLETED if recovers else TaskFinishReason.WINDOW_RECOVERY_TIMEOUT)
        assert cycle_calls == ([0, 0] if recovers else [0])
        assert task.cycles == (1 if recovers else 0)
        assert len(released) >= 2
        assert context.input.backend is original_backend
        assert not any("failed" in event[1].get("message", "") for event in events)

    asyncio.run(exercise())


@pytest.mark.parametrize("task_type", [BlackjackTask, HunterMatchTask, HunterRankTask, HideSeekTask])
def test_exhausted_window_recovery_returns_terminal_reason(task_type):
    async def exercise():
        task = task_type()
        original_backend = object()
        context = SimpleNamespace(input=SimpleNamespace(backend=original_backend, release_all=lambda: None))

        async def capture(*_):
            raise _WindowRecoveryTimeout("300 seconds")

        task._capture = capture
        assert await task.run(context, TaskControl()) == TaskFinishReason.WINDOW_RECOVERY_TIMEOUT
        assert task.cycles == 0
        assert context.input.backend is original_backend

    asyncio.run(exercise())


def test_hide_seek_reroute_preserves_fixed_state_history():
    async def exercise():
        task = HideSeekTask()
        task._fixed_seen = True
        task._escape_active = True

        async def capture(*_):
            return object()

        async def detect(*_):
            assert task._fixed_seen
            assert task._escape_active
            raise _Stopped

        task._capture = capture
        task._detect_stage = detect
        with pytest.raises(_Stopped):
            await task._cycle(None, None)

    asyncio.run(exercise())
