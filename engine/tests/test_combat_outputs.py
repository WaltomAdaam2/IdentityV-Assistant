import asyncio
import random
from types import SimpleNamespace

import pytest

from input.controller import InputController
from core.models import Point
from input.windows_sendinput import WindowsSendInput
from tasks.base import TaskControl
from tasks.blackjack_task import BlackjackTask
from tasks.hunter_match_task import HunterMatchTask, HunterRankTask


class RecordingInput(WindowsSendInput):
    def __init__(self, clock):
        self._expected_hwnd = 123
        self.clock = clock
        self.events = []
        self.position = Point(100, 100)

    def cursor_position(self):
        return self.position

    def move_mouse(self, point):
        self.events.append((self.clock.now, "move", point))
        self.position = point

    def key_down(self, key):
        self.events.append((self.clock.now, "key_down", key))

    def key_up(self, key):
        self.events.append((self.clock.now, "key_up", key))

    def mouse_down(self, button="left"):
        self.events.append((self.clock.now, "mouse_down", button))

    def mouse_up(self, button="left"):
        self.events.append((self.clock.now, "mouse_up", button))

    def move_relative(self, dx, dy):
        self.events.append((self.clock.now, "turn", (dx, dy)))


class VirtualClock:
    def __init__(self):
        self.now = 0.0
        self.waiters = []

    async def sleep(self, seconds):
        future = asyncio.get_running_loop().create_future()
        self.waiters.append((self.now + seconds, future))
        await future

    async def run(self, coroutine, duration):
        runner = asyncio.create_task(coroutine)
        try:
            while not runner.done():
                # Let nested task creation, input cleanup, and gather finish
                # before advancing all concurrent sleepers on the same clock.
                for _ in range(10):
                    await asyncio.sleep(0)
                pending = [(when, future) for when, future in self.waiters if not future.done()]
                if runner.done():
                    break
                assert pending, "combat stalled without a scheduled action"
                next_time = min(when for when, _ in pending)
                if next_time > duration:
                    self.now = duration
                    break
                self.now = next_time
                self.waiters = []
                for when, future in pending:
                    if when <= self.now:
                        future.set_result(None)
                    else:
                        self.waiters.append((when, future))
            if runner.done():
                await runner
        finally:
            runner.cancel()
            await asyncio.gather(runner, return_exceptions=True)


def combat_context(task):
    clock = VirtualClock()
    backend = RecordingInput(clock)
    control = TaskControl()
    window = SimpleNamespace(
        hwnd=123, is_foreground=True, is_minimized=False,
        client_rect=SimpleNamespace(width=1920),
    )

    async def emit(*_args, **_kwargs):
        pass

    ctx = SimpleNamespace(
        input=InputController(backend), cancellation=SimpleNamespace(cancelled=False),
        window=SimpleNamespace(find_window=lambda: window), emit=emit,
        config=SimpleNamespace(automation=SimpleNamespace(
            camera_turn_scale_percent=100, hunter_match_infinite_loop=False,
            hunter_rank_infinite_loop=False, blackjack_infinite_loop=False,
        )),
    )

    async def sleep(_ctx, _control, seconds):
        await task._check(ctx, control)
        await clock.sleep(seconds)
        await task._check(ctx, control)

    task._sleep = sleep
    task._now = lambda _control: clock.now
    return ctx, control, clock, backend


@pytest.mark.parametrize("task_class", [HunterMatchTask, HunterRankTask])
def test_hunter_combat_survives_brief_hud_misses_and_surrenders(task_class):
    async def scenario():
        task = task_class(random.Random(42))
        task.role = "hunter"
        ctx, control, clock, backend = combat_context(task)

        async def capture(*_args):
            return clock.now

        async def confirmed(_ctx, _control, image, name):
            if name == "playing":
                return int(image) % 2 == 0
            return name == "return_lobby" and image >= 365

        task._capture = capture
        task._confirmed = confirmed
        task._displayed_seconds = lambda _image: None
        await clock.run(task._play(ctx, control), 370)
        assert not ctx.input.pressed_keys
        assert not ctx.input.mouse_buttons_down
        return backend.events

    events = asyncio.run(scenario())
    keys = {value for _, action, value in events if action == "key_down"}
    assert {"f", "r", "t", "space", "q", "e", "esc"} <= keys
    assert keys & set("wasd")
    assert {value for _, action, value in events if action == "mouse_down"} == {"left", "right"}
    assert any(action == "turn" and value[0] for _, action, value in events)
    assert all(when >= 360 for when, action, value in events if action == "key_down" and value == "esc")
    # Longest existing interval still produces repeated backend input despite
    # a missing HUD every other observation.
    assert sum(action == "key_down" and value == "t" for _, action, value in events) >= 6
    left = [when for when, action, value in events if action == "mouse_up" and value == "left" and when < 355]
    e = [when for when, action, value in events if action == "key_down" and value == "e"]
    assert all(any(0.3 <= follow - when <= 3.001 for follow in e) for when in left)


@pytest.mark.parametrize("role", ["hunter", "survivor"])
def test_blackjack_sends_space_q_e_and_releases_inputs(role):
    async def scenario():
        task = BlackjackTask(random.Random(42))
        task.role = role
        ctx, control, clock, backend = combat_context(task)
        actions = task._hunter(ctx, control) if role == "hunter" else task._survivor(ctx, control)
        await clock.run(actions, 90)
        assert not ctx.input.pressed_keys
        assert not ctx.input.mouse_buttons_down
        assert not any(not future.done() for _, future in clock.waiters)
        return backend.events

    events = asyncio.run(scenario())
    for key in ("space", "q", "e"):
        downs = [when for when, action, value in events if action == "key_down" and value == key]
        ups = [when for when, action, value in events if action == "key_up" and value == key]
        assert len(downs) >= 3, key
        assert len(downs) == len(ups), key
        if key in ("space", "q"):
            assert all(5 <= later - earlier <= 10.8 for earlier, later in zip(downs, downs[1:]))
    assert any(action == "turn" and value[0] for _, action, value in events)


def test_blackjack_role_change_cancels_hunter_combo_and_extra_keys():
    async def scenario():
        task = BlackjackTask(random.Random(42))
        task.role = "hunter"
        ctx, control, clock, backend = combat_context(task)

        async def capture(*_args):
            return clock.now

        async def confirmed(_ctx, _control, image, name):
            return name == "return_lobby" and image >= 80

        task._capture = capture
        task._confirmed = confirmed
        task.detector = SimpleNamespace(role=lambda image: ("hunter" if image < 21 else "survivor", None))
        await clock.run(task._play(ctx, control), 90)
        assert task.role == "survivor"
        assert not ctx.input.pressed_keys
        assert not ctx.input.mouse_buttons_down
        assert not any(not future.done() for _, future in clock.waiters)
        return backend.events

    events = asyncio.run(scenario())
    assert not any(when > 26 and action == "key_down" and value == "t" for when, action, value in events)
    for key in ("q", "space", "e", "r"):
        assert any(when > 30 and action == "key_down" and value == key for when, action, value in events)


def test_menu_cursor_motion_has_small_even_steps_and_exact_endpoint():
    async def scenario():
        task = BlackjackTask(random.Random(42))
        ctx, control, clock, backend = combat_context(task)
        await clock.run(task._curved_move(ctx, control, Point(1700, 900)), 1)
        assert backend.position == Point(1700, 900)
        assert 0.18 <= clock.now <= 0.851
        events = list(backend.events)
        await clock.run(task._curved_move(ctx, control, Point(1700, 900)), 2)
        assert backend.events == events
        return events

    events = asyncio.run(scenario())
    points = [point for _, action, point in events if action == "move"]
    assert len(points) > 40
    assert all(a != b for a, b in zip(points, points[1:]))
    displacements = [((b.x - a.x) ** 2 + (b.y - a.y) ** 2) ** 0.5 for a, b in zip([Point(100, 100)] + points, points)]
    assert max(displacements) < 55
    assert displacements[0] < max(displacements) / 5
    assert displacements[-1] < max(displacements) / 5
    intervals = [later[0] - earlier[0] for earlier, later in zip(events, events[1:])]
    assert max(intervals) - min(intervals) < 1e-9


@pytest.mark.parametrize("scale", [1, 100])
def test_camera_motion_preserves_distance_and_skips_zero_deltas(scale):
    async def scenario():
        task = BlackjackTask(random.Random(42))
        ctx, control, clock, backend = combat_context(task)
        ctx.config.automation.camera_turn_scale_percent = scale
        await clock.run(task._turn(ctx, control), 1)
        return backend.events, clock.now

    events, duration = asyncio.run(scenario())
    expected_rng = random.Random(42)
    angle = expected_rng.uniform(60, 150)
    direction = expected_rng.choice((-1, 1))
    deltas = [delta[0] for _, action, delta in events if action == "turn"]
    assert deltas
    assert all(delta * direction > 0 for delta in deltas)
    assert sum(deltas) == max(1, round(1920 * angle / 360 * scale / 100)) * direction
    assert duration == pytest.approx(angle / 600)
    assert duration <= 0.25
    if scale == 100:
        assert abs(deltas[0]) < max(map(abs, deltas)) / 3
        assert abs(deltas[-1]) < max(map(abs, deltas)) / 3
        intervals = [later[0] - earlier[0] for earlier, later in zip(events, events[1:])]
        assert max(intervals) - min(intervals) < 1e-9


def test_camera_aborts_stale_turn_after_resume():
    async def scenario():
        task = BlackjackTask(random.Random(42))
        ctx, control, clock, backend = combat_context(task)
        sleep = task._sleep

        async def resumed_sleep(*args):
            await sleep(*args)
            task._resume_recheck = True

        task._sleep = resumed_sleep
        await clock.run(task._turn(ctx, control), 1)
        return backend.events

    events = asyncio.run(scenario())
    assert len(events) == 1
    assert events[0][1] == "turn"


def test_hunter_defers_camera_that_would_delay_followup_e(monkeypatch):
    class LatestFollowup(random.Random):
        def uniform(self, low, high):
            return 3.0 if (low, high) == (0.3, 3.0) else (low + high) / 2

    monkeypatch.setattr("tasks.hunter_match_task.ACTION_INTERVALS", {
        "left": (5, 5), "turn": (7.9, 7.9),
    })

    async def scenario():
        task = HunterMatchTask(LatestFollowup(42))
        ctx, control, clock, backend = combat_context(task)
        await clock.run(task._combat(ctx, control), 9)
        return backend.events

    events = asyncio.run(scenario())
    left_up = next(when for when, action, value in events if action == "mouse_up" and value == "left")
    e_down = next(when for when, action, value in events if action == "key_down" and value == "e")
    camera_start = next(when for when, action, _value in events if action == "turn")
    assert e_down - left_up == pytest.approx(3)
    assert camera_start > e_down
