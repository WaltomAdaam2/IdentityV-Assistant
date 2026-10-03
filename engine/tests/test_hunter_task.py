import asyncio
import random
from types import SimpleNamespace

import cv2
from app.engine import AutomationEngine
from config.loader import parse_config
from vision.hunter_detector import ROOT
from tasks.hunter_match_task import (
    ACTION_INTERVALS,
    HunterMatchTask,
    HunterRankTask,
    SurrenderClock,
    parse_game_clock,
)


NORMAL_STAGES = [
    "home", "diary", "hunter_choice", "start_case", "normal_matching",
    "ready_case", "ready_start", "playing", "result_first",
    "result_second", "return_lobby", "home",
]
RANK_STAGES = [
    "home", "rank_entry", "traditional_rank", "hunter_choice", "start_case",
    "rank_matching", "ready_case", "ready_start", "playing",
    "result_first", "result_second", "return_lobby", "home",
]


def test_hunter_tasks_are_registered_with_independent_loop_defaults() -> None:
    factories = AutomationEngine._task_factories()
    assert factories["hunter_match"] is HunterMatchTask
    assert factories["hunter_rank"] is HunterRankTask
    settings = parse_config({})
    assert settings.automation.hunter_match_infinite_loop is False
    assert settings.automation.hunter_rank_infinite_loop is False


def test_both_hunter_modes_resume_from_each_known_stage() -> None:
    async def scenario(task_class, stages, start):
        task = task_class()
        remaining = iter(stages[start:])
        clicked = []
        played = []

        async def capture(*_args):
            return object()

        async def detect(*_args):
            return next(remaining)

        async def click(_ctx, _control, stage, _action):
            clicked.append(stage)
            return True

        async def play(*_args):
            played.append(True)

        async def noop(*_args):
            pass

        task._capture = capture
        task._detect_stage = detect
        task._click = click
        task._play = play
        task._sleep = noop
        task._progress = noop
        await task._cycle(None, None)
        return clicked, played

    for task_class, stages in ((HunterMatchTask, NORMAL_STAGES), (HunterRankTask, RANK_STAGES)):
        non_actions = {"normal_matching", "rank_matching", "playing"}
        for start in range(len(stages) - 1):
            clicked, played = asyncio.run(scenario(task_class, stages, start))
            assert clicked == [stage for stage in stages[start:-1] if stage not in non_actions]
            assert played == ([True] if start <= stages.index("playing") else [])


def test_known_waiting_screen_does_not_timeout_or_repeat_click() -> None:
    async def scenario():
        task = HunterMatchTask()
        stages = iter(["ready_case"] * 1300 + ["result_first", "result_second", "return_lobby", "home"])
        clicks = []

        async def capture(*_args):
            return object()

        async def detect(*_args):
            return next(stages)

        async def click(_ctx, _control, stage, _action):
            clicks.append(stage)
            return True

        async def noop(*_args):
            pass

        task._capture = capture
        task._detect_stage = detect
        task._click = click
        task._sleep = noop
        task._progress = noop
        await task._cycle(None, None)
        return clicks

    assert asyncio.run(scenario()) == ["ready_case", "result_first", "result_second", "return_lobby"]


def test_surrender_uses_game_clock_then_local_fallback() -> None:
    assert parse_game_clock("4:23") == 263
    assert parse_game_clock("06：00") == 360
    assert parse_game_clock("match 01:99") is None
    clock = SurrenderClock.start(100.0)
    assert not clock.due(450.0, 359)
    assert clock.due(451.0, 360)
    clock = SurrenderClock.start(100.0)
    assert not clock.due(459.0, None)
    assert clock.due(460.0, None)
    clock = SurrenderClock.start(100.0)
    assert not clock.due(460.0, 359)
    assert not clock.due(489.0, None)
    assert clock.due(490.0, None)


def test_limited_ocr_region_reads_supplied_game_timer() -> None:
    screenshot = cv2.imread(str(ROOT / "hunter_match" / "reference" / "surrender-menu.png"))
    assert screenshot is not None
    assert HunterMatchTask()._displayed_seconds(screenshot[45:1125, 2:1922]) == 263


def test_stage_requires_two_observations() -> None:
    async def scenario(second_found):
        task = HunterMatchTask()
        seen = []
        detector = SimpleNamespace(detect=lambda image, _name: SimpleNamespace(found=image, confidence=1.0, bbox=None))
        task.detector = detector

        async def capture(*_args):
            return second_found

        async def pause(_ctx, _control, seconds):
            seen.append(seconds)

        async def emit(*_args, **_kwargs):
            seen.append("evidence")

        task._capture = capture
        task._sleep = pause
        found = await task._confirmed(SimpleNamespace(emit=emit), None, True, "diary")
        return found, seen

    assert asyncio.run(scenario(False)) == (False, [0.01])
    assert asyncio.run(scenario(True)) == (True, [0.01, "evidence"])


def test_leaving_play_releases_all_keys() -> None:
    async def scenario():
        task = HunterMatchTask()
        released = []

        async def idle(*_args):
            await asyncio.Event().wait()

        async def capture(*_args):
            return object()

        async def confirmed(_ctx, _control, _image, _name):
            return False

        async def pause(*_args):
            await asyncio.sleep(0)

        task._movement = idle
        task._combat = idle
        task._displayed_seconds = lambda _image: None
        task._capture = capture
        task._confirmed = confirmed
        task._sleep = pause
        ctx = SimpleNamespace(input=SimpleNamespace(release_all=lambda: released.append(True)))
        await task._play(ctx, None)
        return released

    assert asyncio.run(scenario()) == [True]


def test_hunter_action_intervals_and_left_followed_by_e() -> None:
    class Done(Exception):
        pass

    async def scenario():
        task = HunterMatchTask(random.Random(42))
        elapsed = 0.0
        events = []

        async def sleep(_ctx, _control, seconds):
            nonlocal elapsed
            elapsed += seconds
            if elapsed > 800:
                raise Done

        async def key(_ctx, _control, name):
            events.append((name, elapsed))

        async def click(_ctx, _control, name):
            events.append((name, elapsed))

        async def turn(_ctx, _control):
            events.append(("turn", elapsed))

        task._sleep = sleep
        task._now = lambda _control: elapsed
        task._key = key
        task._mouse_click = click
        task._turn = turn
        try:
            await task._combat(None, None)
        except Done:
            pass
        return events

    events = asyncio.run(scenario())
    assert len(events) > 50
    for name, (minimum, maximum) in ACTION_INTERVALS.items():
        times = [when for event, when in events if event == name]
        assert times
        assert all(minimum <= later - earlier <= maximum + 0.001 for earlier, later in zip(times, times[1:]))
    left_times = [when for event, when in events if event == "left"]
    e_times = [when for event, when in events if event == "e"]
    assert all(any(0.3 <= e - left <= 3.001 for e in e_times) for left in left_times[:-1])
