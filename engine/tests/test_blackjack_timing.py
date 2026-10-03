import asyncio
import random

from tasks.blackjack_task import BlackjackTask, SURVIVOR_INTERVALS, _Stopped, hunter_timeline


def test_hunter_timeline_stays_within_requested_ranges() -> None:
    rng = random.Random(42)
    for _ in range(100):
        t, left, e, cooldown = hunter_timeline(rng)
        assert 3.5 <= t <= 4.5
        assert 5 <= left <= 8
        assert t < left
        assert 5 <= e <= 7
        assert 10 <= cooldown <= 12


def test_survivor_intervals_and_hunter_combo_with_virtual_time() -> None:
    async def scenario() -> tuple[list[tuple[float, str]], list[tuple[float, str]]]:
        survivor = BlackjackTask(random.Random(42))
        hunter = BlackjackTask(random.Random(42))
        current = [0.0]
        survivor_events: list[tuple[float, str]] = []
        hunter_events: list[tuple[float, str]] = []

        async def sleep(_ctx, _control, seconds):
            current[0] += seconds
            if current[0] > 70:
                raise _Stopped

        async def survivor_mouse(_ctx, _control, button):
            survivor_events.append((current[0], button))

        async def survivor_key(_ctx, _control, key):
            survivor_events.append((current[0], key))

        async def survivor_turn(_ctx, _control):
            survivor_events.append((current[0], "turn"))

        survivor._sleep = sleep
        survivor._now = lambda _control: current[0]
        survivor._mouse_click = survivor_mouse
        survivor._key = survivor_key
        survivor._turn = survivor_turn
        try:
            await survivor._survivor(None, None)
        except _Stopped:
            pass

        current[0] = 0.0

        async def hunter_mouse(_ctx, _control, button):
            hunter_events.append((current[0], button))

        async def hunter_key(_ctx, _control, key):
            hunter_events.append((current[0], key))

        async def hunter_turn(_ctx, _control):
            hunter_events.append((current[0], "turn"))

        hunter._sleep = sleep
        hunter._mouse_click = hunter_mouse
        hunter._key = hunter_key
        hunter._turn = hunter_turn
        try:
            await hunter._hunter_combo(None, None)
        except _Stopped:
            pass
        return survivor_events, hunter_events

    survivor, hunter = asyncio.run(scenario())
    for name, bounds in SURVIVOR_INTERVALS.items():
        observed = [(time, action) for time, action in survivor if action == ("r" if name == "r_double" else name)]
        assert observed, name
        if name == "r_double":
            for first, second in zip(observed[::2], observed[1::2]):
                assert 0.4 <= second[0] - first[0] <= 0.6
        else:
            for first, second in zip(observed, observed[1:]):
                assert bounds[0] <= second[0] - first[0] <= bounds[1] + 0.6
    assert hunter[0][1] == "right"
    assert [name for _, name in hunter[:5]] == ["right", "t", "left", "e", "turn"]
    assert 3.5 <= hunter[1][0] - hunter[0][0] <= 4.5
    assert 5 <= hunter[2][0] - hunter[0][0] <= 8
    assert 5 <= hunter[3][0] - hunter[2][0] <= 7
