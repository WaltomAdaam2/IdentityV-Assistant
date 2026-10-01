import asyncio

from tasks.blackjack_task import BlackjackTask


STAGES = [
    "home", "chaos", "blackjack", "start_case", "matching", "ready_case",
    "ready_start", "card_confirm", "hunter", "continue", "return_lobby", "home",
]


def test_blackjack_resumes_from_each_known_stage() -> None:
    async def scenario(start: int):
        task = BlackjackTask()
        stages = iter(STAGES[start:])
        clicked = []
        played = []

        async def capture(_ctx, _control):
            return object()

        async def detect(_ctx, _control, _image):
            return next(stages)

        async def click(_ctx, _control, stage, _action):
            clicked.append(stage)
            return True

        async def play(_ctx, _control):
            played.append(task.role)

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

    for start in range(len(STAGES) - 1):
        clicked, played = asyncio.run(scenario(start))
        assert clicked == [stage for stage in STAGES[start:-1] if stage in {
            "home", "chaos", "blackjack", "start_case", "ready_case", "ready_start",
            "card_confirm", "continue", "return_lobby",
        }]
        assert played == (["hunter"] if start <= STAGES.index("hunter") else [])


def test_waiting_screen_is_not_timed_out_or_clicked_repeatedly() -> None:
    async def scenario():
        task = BlackjackTask()
        stages = iter(["ready_case"] * 1300 + ["continue", "return_lobby", "home"])
        clicks = []

        async def capture(_ctx, _control):
            return object()

        async def detect(_ctx, _control, _image):
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

    assert asyncio.run(scenario()) == ["ready_case", "continue", "return_lobby"]
