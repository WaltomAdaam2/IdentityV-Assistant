import asyncio
import random
from types import SimpleNamespace

import pytest

from tasks.base import TaskControl
from tasks.hide_seek_task import HideSeekTask, camera_timing, movement_timing


STAGES = [
    "home", "chaos", "hide_seek", "start_game", "matching", "ready_case",
    "ready_start", "f_initial", "v_ready", "v_fixed", "death_f", "continue",
    "return_lobby", "home",
]


def test_task_is_registered_and_loop_setting_is_independent():
    from app.engine import AutomationEngine
    from config.loader import parse_config

    assert AutomationEngine._task_factories()["hide_seek"] is HideSeekTask
    assert not parse_config({}).automation.hide_seek_infinite_loop
    config = parse_config({"automation": {"hide_seek_infinite_loop": True}})
    assert config.automation.hide_seek_infinite_loop
    assert not config.automation.blackjack_infinite_loop
    assert not config.automation.hunter_match_infinite_loop


def test_resume_from_each_stage_performs_only_remaining_actions():
    async def scenario(start):
        task = HideSeekTask()
        stages = iter(STAGES[start:])
        clicked, played, keys = [], [], []

        async def capture(*_):
            return object()

        async def detect(*_):
            return next(stages)

        async def click(_ctx, _control, stage, _action):
            clicked.append(stage)
            return True

        async def play(*_):
            played.append(task.stage)

        async def key(_ctx, _control, value):
            keys.append(value)

        async def progress(_ctx, value):
            task.stage = value

        async def noop(*_):
            pass

        task._capture, task._detect_stage, task._click = capture, detect, click
        task._play, task._key, task._progress, task._sleep = play, key, progress, noop
        ctx = SimpleNamespace(input=SimpleNamespace(release_all=lambda: None))
        await task._cycle(ctx, None)
        return clicked, played, keys

    actions = {"home", "chaos", "hide_seek", "ready_case", "ready_start", "continue", "return_lobby"}
    for start in range(len(STAGES) - 1):
        clicked, played, keys = asyncio.run(scenario(start))
        expected = []
        for stage in STAGES[start:-1]:
            if stage == "start_game":
                expected += ["bad_child", "start_game"]
            elif stage in actions:
                expected.append(stage)
        assert clicked == expected
        assert played == [s for s in STAGES[start:-1] if s in {"f_initial", "v_ready", "v_fixed"}]
        assert keys == (["esc"] if start <= STAGES.index("death_f") else [])


def test_known_waits_and_death_do_not_repeat_actions_or_timeout():
    async def scenario():
        task = HideSeekTask()
        stages = iter(["ready_case"] * 1300 + ["death_f"] * 1300 + ["continue", "return_lobby", "home"])
        actions = []

        async def capture(*_):
            return object()

        async def detect(*_):
            return next(stages)

        async def click(_ctx, _control, stage, _action):
            actions.append(stage)
            return True

        async def key(_ctx, _control, value):
            actions.append(value)

        async def noop(*_):
            pass

        task._capture, task._detect_stage, task._click = capture, detect, click
        task._key, task._progress, task._sleep = key, noop, noop
        await task._cycle(SimpleNamespace(input=SimpleNamespace(release_all=lambda: None)), None)
        return actions

    assert asyncio.run(scenario()) == ["ready_case", "esc", "continue", "return_lobby"]


def test_confirmation_requires_matching_second_frame_after_ten_ms():
    async def scenario(second):
        task = HideSeekTask()
        events = []
        task.detector = SimpleNamespace(detect=lambda frame, _: SimpleNamespace(found=frame, confidence=1, bbox=None))

        async def capture(*_):
            return second

        async def sleep(_ctx, _control, seconds):
            events.append(seconds)

        async def emit(*_, **__):
            events.append("evidence")

        async def check(*_):
            pass

        task._capture, task._sleep, task._check = capture, sleep, check
        return await task._confirmed(SimpleNamespace(emit=emit), None, True, "v_fixed"), events

    assert asyncio.run(scenario(False)) == (False, [0.01])
    assert asyncio.run(scenario(True)) == (True, [0.01, "evidence"])


def test_fixed_seed_timing_ranges_and_transform_order():
    rng = random.Random(12)
    for _ in range(1000):
        duration, gap = movement_timing(rng)
        assert 13 <= duration <= 17 and 1 <= gap <= 5
        duration, gap = camera_timing(rng)
        assert 2.5 <= duration <= 3.5 and 2.5 <= gap <= 3.5

    async def scenario(stage, blocked=True):
        task = HideSeekTask(random.Random(12))
        events = []

        async def hold(_ctx, _control, seconds):
            events.append(seconds)

        async def key(_ctx, _control, name):
            events.append(name)

        async def capture(*_):
            return object()

        async def confirmed(*_):
            return blocked

        task._hold, task._key, task._capture, task._confirmed = hold, key, capture, confirmed
        await task._prepare_hiding(None, None, stage)
        return events

    initial = asyncio.run(scenario("f_initial"))
    assert 1 <= initial[0] <= 3 and initial[1] == "f"
    transformed = asyncio.run(scenario("v_ready"))
    assert 5 <= transformed[0] <= 8 and transformed[1] == "v"
    assert asyncio.run(scenario("v_ready", False)) == []


def test_leaving_play_cancels_movement_and_releases_all_input():
    async def scenario():
        task = HideSeekTask()
        stages = iter(["v_fixed", "loading"])
        released = []
        cancelled = []

        async def capture(*_):
            return object()

        async def detect(*_):
            return next(stages)

        async def fixed(*_):
            try:
                await asyncio.Event().wait()
            finally:
                cancelled.append(True)

        async def sleep(*_):
            await asyncio.sleep(0)

        async def noop(*_):
            pass

        task._capture, task._detect_stage, task._fixed_actions = capture, detect, fixed
        task._sleep, task._progress = sleep, noop
        await task._play(SimpleNamespace(input=SimpleNamespace(release_all=lambda: released.append(True))), None)
        return released, cancelled

    released, cancelled = asyncio.run(scenario())
    assert released and cancelled == [True]


def test_unknown_and_loading_timeout_use_active_virtual_time():
    async def scenario(stage):
        now = [0.0]
        task = HideSeekTask(clock=lambda: now[0])

        async def capture(*_):
            return object()

        async def detect(*_):
            return stage

        async def noop(*_):
            now[0] += 0.25

        task._capture, task._detect_stage, task._sleep, task._progress = capture, detect, noop, noop
        with pytest.raises(TimeoutError):
            await task._cycle(SimpleNamespace(input=SimpleNamespace(release_all=lambda: None)), None)

    asyncio.run(scenario(None))
    asyncio.run(scenario("loading"))


def test_pause_releases_input_and_resume_requests_screen_recheck():
    async def scenario():
        task = HideSeekTask()
        control = TaskControl()
        control.pause()
        released = []
        ctx = SimpleNamespace(
            cancellation=SimpleNamespace(cancelled=False),
            input=SimpleNamespace(release_all=lambda: released.append(True)),
        )
        pending = asyncio.create_task(task._check(ctx, control))
        await asyncio.sleep(0)
        assert released == [True] and not pending.done()
        control.resume()
        await pending
        assert task._resume_recheck

    asyncio.run(scenario())


def test_supplied_game_frames_route_to_fixed_and_transformed_stages():
    import cv2
    from vision.hide_seek_detector import ROOT

    async def scenario(screen):
        task = HideSeekTask()
        image = cv2.imread(str(ROOT / "reference" / f"{screen}.png"))[45:1125, 2:1922]

        async def capture(*_):
            return image

        async def noop(*_, **__):
            pass

        task._capture, task._check, task._sleep = capture, noop, noop
        return await task._detect_stage(SimpleNamespace(emit=noop), None, image)

    assert asyncio.run(scenario("v-ready")) == "v_ready"
    assert asyncio.run(scenario("v-fixed")) == "v_fixed"
    assert asyncio.run(scenario("eliminated")) == "death_f"
    assert asyncio.run(scenario("result-continue")) == "continue"
    assert asyncio.run(scenario("return-lobby")) == "return_lobby"
