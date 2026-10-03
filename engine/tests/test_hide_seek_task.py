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
        ctx, events = input_context()
        ctx.cancellation = SimpleNamespace(cancelled=False)
        ctx.input.key_down("alt")
        ctx.input.key_down("1")
        release_all = ctx.input.release_all

        def release():
            released.append(True)
            release_all()

        ctx.input.release_all = release
        pending = asyncio.create_task(task._check(ctx, control))
        await asyncio.sleep(0)
        assert released == [True] and not pending.done()
        assert not ctx.input.pressed_keys
        assert ("up", "alt") in events and ("up", "1") in events
        control.resume()
        await pending
        assert task._resume_recheck

    asyncio.run(scenario())


def test_supplied_game_frames_route_to_fixed_and_transformed_stages():
    import cv2
    from vision.hide_seek_detector import ROOT

    async def scenario(screen, fixed_seen=False):
        task = HideSeekTask()
        task._fixed_seen = fixed_seen
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
    assert asyncio.run(scenario("f-initial")) == "f_initial"
    assert asyncio.run(scenario("living-no-v", True)) == "living_no_v"
    assert asyncio.run(scenario("living-no-v")) == "f_initial"


def input_context():
    from input.controller import InputController

    events = []
    backend = SimpleNamespace(
        key_down=lambda key: events.append(("down", key)),
        key_up=lambda key: events.append(("up", key)),
        move_relative=lambda x, y: events.append(("move", x, y)),
    )
    return SimpleNamespace(
        input=InputController(backend),
        config=SimpleNamespace(automation=SimpleNamespace(camera_turn_scale_percent=100)),
    ), events


def test_camera_emits_relative_motion_before_movement_wait_finishes():
    async def scenario():
        task = HideSeekTask(random.Random(12))
        ctx, events = input_context()
        camera_moved = asyncio.Event()

        async def allowed(*_):
            return True

        async def check(*_):
            pass

        async def window(*_):
            return SimpleNamespace(client_rect=SimpleNamespace(width=1920))

        async def sleep(_ctx, _control, seconds):
            if seconds < 1 and any(event[0] == "move" for event in events):
                camera_moved.set()
            if seconds >= 1 or camera_moved.is_set():
                await asyncio.Event().wait()
            await asyncio.sleep(0)

        task._before_game_input, task._input_window, task._sleep = allowed, window, sleep
        task._check = check
        pending = asyncio.create_task(task._fixed_actions(ctx, None))
        await asyncio.wait_for(camera_moved.wait(), 1)
        pending.cancel()
        await asyncio.gather(pending, return_exceptions=True)
        assert any(event[0] == "move" and event[1] != 0 for event in events)
        assert not any(event[0] == "down" for event in events)
        assert not ctx.input.pressed_keys

    asyncio.run(scenario())


@pytest.mark.parametrize("direction", [-1, 1])
@pytest.mark.parametrize("angle", [60.1, 120.0, 199.9])
def test_camera_eases_each_sweep_with_requested_amplitude_and_unchanged_timing(direction, angle):
    async def scenario():
        ranges = []

        def uniform(low, high):
            ranges.append((low, high))
            return 3.0 if (low, high) == (2.5, 3.5) else angle

        task = HideSeekTask(SimpleNamespace(uniform=uniform, choice=lambda _: direction))
        ctx, _ = input_context()
        elapsed = [0.0]
        moves = []
        delays = []
        finished_sweep = asyncio.Event()
        gates, windows, checks = [], [], []
        ctx.input.backend.move_relative = lambda x, y: moves.append((elapsed[0], x, y))

        async def allowed(*_):
            gates.append(True)
            return True

        async def check(*_):
            checks.append(True)

        async def window(*_):
            windows.append(True)
            return SimpleNamespace(client_rect=SimpleNamespace(width=1920))

        async def sleep(_ctx, _control, seconds):
            delays.append(seconds)
            if seconds >= 1:
                finished_sweep.set()
                await asyncio.Event().wait()
            elapsed[0] += seconds
            await asyncio.sleep(0)

        task._before_game_input, task._input_window, task._sleep = allowed, window, sleep
        task._check = check
        pending = asyncio.create_task(task._camera(ctx, None))
        await asyncio.wait_for(finished_sweep.wait(), 1)
        pending.cancel()
        await asyncio.gather(pending, return_exceptions=True)
        distance = round(1920 * angle / 360)
        assert sum(x for _, x, _ in moves) == direction * distance
        assert all(0 < x * direction <= round(distance / 60 * 1.5) + 1 and y == 0 for _, x, y in moves)
        assert 60 < ranges[-1][0] < ranges[-1][1] < 200
        early = sum(abs(x) for time, x, _ in moves if time < 0.75)
        middle = sum(abs(x) for time, x, _ in moves if 0.75 <= time < 1.5)
        late = sum(abs(x) for time, x, _ in moves if time >= 2.25)
        assert early < middle and late < middle
        assert sum(delays[:-1]) == pytest.approx(3.0)
        assert delays[-1] == 3.0
        assert len(gates) == len(windows) == 1
        assert len(checks) == len(delays) - 1

    asyncio.run(scenario())


def test_camera_aborts_sweep_after_pause_requires_screen_recheck():
    async def scenario():
        task = HideSeekTask(SimpleNamespace(uniform=lambda low, _: 3.0 if low == 2.5 else 120.0, choice=lambda _: 1))
        ctx, events = input_context()
        checks = []

        async def allowed(*_):
            return True

        async def window(*_):
            return SimpleNamespace(client_rect=SimpleNamespace(width=1920))

        async def check(*_):
            checks.append(True)
            if len(checks) == 20:
                task._resume_recheck = True

        async def sleep(*_):
            await asyncio.sleep(0)

        task._before_game_input, task._input_window = allowed, window
        task._check, task._sleep = check, sleep
        await asyncio.wait_for(task._camera(ctx, None), 1)
        assert len(checks) == 20
        assert 0 < sum(event[1] for event in events if event[0] == "move") < 640

    asyncio.run(scenario())


def test_escape_emits_ftg_once_and_repeated_alt_one_with_interval_and_cleanup():
    async def scenario():
        now = [0.0]
        task = HideSeekTask(random.Random(12), clock=lambda: now[0])
        ctx, events = input_context()
        delays = []
        gaps = []
        second_alt = asyncio.Event()

        async def allowed(*_):
            return True

        async def sleep(_ctx, _control, seconds):
            now[0] += seconds
            if seconds > 1:
                delays.append(seconds)
            elif seconds > 0.1:
                gaps.append(seconds)
            if events.count(("down", "1")) == 2:
                second_alt.set()
                await asyncio.Event().wait()
            await asyncio.sleep(0)

        task._before_game_input, task._sleep = allowed, sleep
        pending = asyncio.create_task(task._escape_actions(ctx, None))
        await asyncio.wait_for(second_alt.wait(), 1)
        pending.cancel()
        await asyncio.gather(pending, return_exceptions=True)
        assert events == [(action, key) for key in ("f", "t", "g") for action in ("down", "up")] + [
            ("down", "alt"), ("down", "1"), ("up", "1"), ("up", "alt"),
            ("down", "alt"), ("down", "1"), ("up", "1"), ("up", "alt"),
        ]
        assert len(delays) == 2 and all(7 <= delay <= 13 for delay in delays)
        assert len(gaps) == 2 and all(0.3 <= gap <= 0.7 for gap in gaps)
        assert not ctx.input.pressed_keys

    asyncio.run(scenario())


def test_alt_one_observes_modifier_lead_digit_hold_and_modifier_trail():
    async def scenario():
        now = [0.0]
        task = HideSeekTask(random.Random(12), clock=lambda: now[0])
        task._escape_step = 3
        task._next_alt_at = 0.0
        ctx, _ = input_context()
        events = []
        gates = []
        ctx.input.backend.key_down = lambda key: events.append(("down", key, now[0]))
        ctx.input.backend.key_up = lambda key: events.append(("up", key, now[0]))

        async def allowed(*_):
            gates.append(True)
            return len(gates) == 1

        async def sleep(_ctx, _control, seconds):
            now[0] += seconds

        task._before_game_input, task._sleep = allowed, sleep
        await task._escape_actions(ctx, None)
        assert [(action, key) for action, key, _ in events] == [
            ("down", "alt"), ("down", "1"), ("up", "1"), ("up", "alt"),
        ]
        assert [time for _, _, time in events] == pytest.approx([0.0, 0.08, 0.18, 0.24])
        assert not ctx.input.pressed_keys

    asyncio.run(scenario())


def test_pause_during_alt_lead_releases_modifier_without_sending_unmodified_digit():
    async def scenario():
        task = HideSeekTask(clock=lambda: 0.0)
        task._escape_step = 3
        task._next_alt_at = 0.0
        ctx, events = input_context()
        ctx.cancellation = SimpleNamespace(cancelled=False)
        control = TaskControl()

        async def allowed(*_):
            await task._check(ctx, control)
            return not task._resume_recheck

        async def sleep(_ctx, _control, seconds):
            if seconds == 0.08:
                assert ctx.input.pressed_keys == {"alt"}
                control.pause()
                check = asyncio.create_task(task._check(ctx, control))
                await asyncio.sleep(0)
                assert not ctx.input.pressed_keys and not check.done()
                control.resume()
                await check

        task._before_game_input, task._sleep = allowed, sleep
        await asyncio.wait_for(task._escape_actions(ctx, control), 1)
        assert task._resume_recheck
        assert ("down", "alt") in events and ("up", "alt") in events
        assert ("down", "1") not in events
        assert not ctx.input.pressed_keys

    asyncio.run(scenario())


@pytest.mark.parametrize("ending", ["death_f", "loading", "continue", "return_lobby", None])
def test_post_fixed_stage_changes_keep_camera_running_and_only_living_no_v_triggers_escape(ending):
    async def scenario(stages):
        task = HideSeekTask()
        values = iter(stages)
        started = []
        cancelled = []

        async def capture(*_):
            return object()

        async def detect(*_):
            return next(values)

        async def action(name):
            started.append(name)
            try:
                await asyncio.Event().wait()
            finally:
                cancelled.append(name)

        async def fixed(*_):
            await action("fixed")

        async def escape(*_):
            await action("escape")

        async def sleep(*_):
            await asyncio.sleep(0)

        async def progress(_ctx, stage):
            task.stage = stage

        task._capture, task._detect_stage = capture, detect
        task._fixed_actions, task._escape_actions = fixed, escape
        task._sleep, task._progress = sleep, progress
        ctx, _ = input_context()
        await task._play(ctx, None)
        assert sorted(started) == sorted(cancelled)
        assert not ctx.input.pressed_keys
        return started, task._fixed_seen

    started, fixed_seen = asyncio.run(scenario(["v_fixed", "r_blocked", "v_ready", "living_no_v", "living_no_v", ending]))
    assert started == ["fixed", "escape"]
    assert fixed_seen == (ending is None)
    started, _ = asyncio.run(scenario(["v_fixed", ending]))
    assert started == ["fixed"]
    started, _ = asyncio.run(scenario(["living_no_v", ending]))
    assert started == []


def test_resume_escape_after_unknown_does_not_repeat_ftg():
    async def scenario():
        task = HideSeekTask(random.Random(12), clock=lambda: 0)
        ctx, events = input_context()
        waiting = asyncio.Event()

        async def allowed(*_):
            return True

        async def sleep(_ctx, _control, seconds):
            if seconds > 1:
                waiting.set()
                await asyncio.Event().wait()

        task._before_game_input, task._sleep = allowed, sleep
        for _ in range(2):
            waiting.clear()
            pending = asyncio.create_task(task._escape_actions(ctx, None))
            await asyncio.wait_for(waiting.wait(), 1)
            pending.cancel()
            await asyncio.gather(pending, return_exceptions=True)
        assert events == [(action, key) for key in ("f", "t", "g") for action in ("down", "up")]
        assert not ctx.input.pressed_keys

    asyncio.run(scenario())


def test_v_disappearance_keeps_wasd_held_while_ftg_runs_then_releases_on_death():
    async def scenario():
        task = HideSeekTask(random.Random(12))
        ctx, events = input_context()
        movement_started = asyncio.Event()
        combo_finished = asyncio.Event()
        held_during_taps = []
        fixed_starts = []
        backend_key_down = ctx.input.backend.key_down

        def key_down(key):
            backend_key_down(key)
            if key in "wasd":
                movement_started.set()
            elif key in ("f", "t", "g"):
                held_during_taps.append((key, ctx.input.pressed_keys & set("wasd")))

        ctx.input.backend.key_down = key_down

        async def allowed(*_):
            return True

        async def capture(*_):
            return object()

        async def detect(*_):
            if not fixed_starts:
                return "v_fixed"
            await movement_started.wait()
            return "death_f" if combo_finished.is_set() else "living_no_v"

        async def fixed(*_):
            fixed_starts.append(True)
            await task._hold(ctx, None, 15)

        async def sleep(_ctx, _control, seconds):
            if seconds >= 7:
                if seconds < 15:
                    combo_finished.set()
                await asyncio.Event().wait()
            await asyncio.sleep(0)

        async def progress(_ctx, stage):
            task.stage = stage

        task._before_game_input, task._capture, task._detect_stage = allowed, capture, detect
        task._fixed_actions, task._sleep, task._progress = fixed, sleep, progress
        await asyncio.wait_for(task._play(ctx, None), 1)
        assert len(fixed_starts) == 1
        assert [key for key, _ in held_during_taps] == ["f", "t", "g"]
        movement_key = next(key for action, key in events if action == "down" and key in "wasd")
        assert all(held == {movement_key} for _, held in held_during_taps)
        assert events.count(("down", movement_key)) == 1
        assert events.index(("up", movement_key)) > events.index(("up", "g"))
        assert not ctx.input.pressed_keys

    asyncio.run(scenario())
