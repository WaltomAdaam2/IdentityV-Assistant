from __future__ import annotations

import asyncio
from types import SimpleNamespace

import pytest

from core.models import Point, Rect
from tasks.base import TaskControl
from tasks.login_lobby_task import LoginLobbyTask, _ActiveDeadline, _TaskStopped
from window.base import WindowInfo


class _Input:
    def __init__(self) -> None:
        self.clicks: list[Point] = []
        self.releases = 0

    def click(self, point: Point, bounds: Rect | None = None) -> None:
        self.clicks.append(point)

    def release_all(self) -> None:
        self.releases += 1


def _context(*, click_delay_ms: int = 100):
    input_controller = _Input()
    window = WindowInfo(42, "Game", "Class", 1, Rect(0, 0, 1920, 1080), 1.0)

    async def emit(*args, **kwargs) -> None:
        pass

    return SimpleNamespace(
        config=SimpleNamespace(
            automation=SimpleNamespace(
                click_after_detection_ms=click_delay_ms,
                click_interval_jitter_ms=0,
                screenshot_after_click_ms=10,
            ),
            vision=SimpleNamespace(state_confirmation_count=2),
        ),
        window=SimpleNamespace(find_window=lambda: window),
        capture=SimpleNamespace(capture=lambda rect, hwnd: SimpleNamespace(image=object())),
        metrics=SimpleNamespace(screenshots=0),
        input=input_controller,
        cancellation=SimpleNamespace(cancelled=False),
        emit=emit,
    )


def test_pause_freezes_login_delay_deadline() -> None:
    async def scenario() -> tuple[float, int]:
        ctx = _context()
        control = TaskControl()
        deadline = _ActiveDeadline(asyncio.get_running_loop().time() + 0.08)
        original = deadline.expires_at
        waiting = asyncio.create_task(
            LoginLobbyTask._random_delay(ctx, control, 60, 0, deadline)
        )
        await asyncio.sleep(0.01)
        control.pause()
        await asyncio.sleep(0.10)
        control.resume()
        await waiting
        return deadline.expires_at - original, ctx.input.releases

    extension, releases = asyncio.run(scenario())

    assert extension >= 0.08
    assert releases >= 1


def test_stop_during_login_click_delay_prevents_click() -> None:
    async def scenario() -> list[Point]:
        ctx = _context(click_delay_ms=250)
        control = TaskControl()
        task = LoginLobbyTask()
        deadline = _ActiveDeadline(asyncio.get_running_loop().time() + 1)
        clicking = asyncio.create_task(
            task._click(
                ctx,
                control,
                deadline,
                "login_landing",
                Point(960, 540),
                "enter_game",
            )
        )
        await asyncio.sleep(0.02)
        control.cancel()
        with pytest.raises(_TaskStopped):
            await clicking
        return ctx.input.clicks

    assert asyncio.run(scenario()) == []


def test_resume_rechecks_screen_before_login_click() -> None:
    async def scenario() -> tuple[bool, list[Point]]:
        ctx = _context(click_delay_ms=100)
        control = TaskControl()
        task = LoginLobbyTask()
        stage = ["login_landing"]
        task._detect_stage = lambda image: stage[0]  # type: ignore[method-assign]
        deadline = _ActiveDeadline(asyncio.get_running_loop().time() + 1)
        clicking = asyncio.create_task(
            task._click(
                ctx,
                control,
                deadline,
                "login_landing",
                Point(960, 540),
                "enter_game",
            )
        )
        await asyncio.sleep(0.02)
        control.pause()
        stage[0] = "announcement"
        await asyncio.sleep(0.06)
        control.resume()
        return await clicking, ctx.input.clicks

    clicked, clicks = asyncio.run(scenario())

    assert not clicked
    assert clicks == []
