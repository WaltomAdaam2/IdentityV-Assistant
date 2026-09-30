from __future__ import annotations

import asyncio
import random
from time import monotonic
from typing import Any

from app.context import AutomationContext
from core.enums import TaskFinishReason
from core.models import CoordinateSpace, Point
from tasks.base import AutomationTask, TaskControl
from vision.home_detector import detect_home_book
from vision.login_detector import is_activity_popup, is_announcement, is_login_landing, is_platform_choice


class _TaskStopped(Exception):
    pass


class LoginLobbyTask(AutomationTask):
    """Select a login platform, dismiss startup pages, and wait for the home lobby."""

    async def run(self, ctx: AutomationContext, control: TaskControl) -> TaskFinishReason:
        deadline = monotonic() + ctx.config.automation.login_timeout_seconds
        try:
            await self._wait_for_window(ctx, control, deadline)
            await self._random_delay(350, ctx.config.automation.click_interval_jitter_ms, deadline)
            last_action_stage = ""
            last_action_at = 0.0
            while True:
                stage, window = await self._wait_for_confirmed_stage(ctx, control, deadline)
                if stage == "home_lobby":
                    break
                if stage == last_action_stage and monotonic() - last_action_at < 2.0:
                    await self._random_delay(250, 50, deadline)
                    continue
                point, action = self._stage_action(ctx, stage)
                await self._click(ctx, deadline, window.client_rect, point, action)
                last_action_stage = stage
                last_action_at = monotonic()
        except _TaskStopped:
            return TaskFinishReason.USER_STOPPED
        except TimeoutError as error:
            await ctx.emit(
                "log",
                level="warning",
                source="login_lobby",
                message=f"Login timed out while waiting for {error}",
                data={"important": True},
            )
            return TaskFinishReason.TIMEOUT
        except Exception as error:
            await ctx.emit(
                "log",
                level="error",
                source="login_lobby",
                message=f"Login automation failed: {error}",
                data={"important": True},
            )
            return TaskFinishReason.ENGINE_ERROR

        await ctx.emit(
            "log",
            level="info",
            source="login_lobby",
            message="Login completed and the home lobby was recognized",
            data={"important": True},
        )
        return TaskFinishReason.COMPLETED

    async def _wait_for_window(self, ctx: AutomationContext, control: TaskControl, deadline: float):
        while monotonic() < deadline:
            await self._check_control(ctx, control)
            window = ctx.window.find_window()
            if window is not None and window.is_responding:
                return window
            await self._random_delay(500, ctx.config.automation.click_interval_jitter_ms, deadline)
        raise TimeoutError("game_window")

    async def _wait_for_confirmed_stage(
        self,
        ctx: AutomationContext,
        control: TaskControl,
        deadline: float,
    ):
        while monotonic() < deadline:
            await self._check_control(ctx, control)
            window = ctx.window.find_window()
            if window is not None and window.is_responding and not window.is_minimized:
                ctx.memory_guard.protect()
                try:
                    frame = ctx.capture.capture(window.client_rect, window.hwnd)
                except Exception:
                    frame = None
                if frame is not None:
                    ctx.metrics.screenshots += 1
                    stage = self._detect_stage(frame.image)
                    if stage and await self._confirm_stage(
                        ctx, control, deadline, window.hwnd, stage
                    ):
                        await ctx.emit("login_stage", stage=stage, found=True)
                        current = ctx.window.find_window()
                        if current is not None:
                            return stage, current
            await self._random_delay(
                250,
                ctx.config.automation.click_interval_jitter_ms,
                deadline,
            )
        raise TimeoutError("a recognizable login screen")

    async def _confirm_stage(
        self,
        ctx: AutomationContext,
        control: TaskControl,
        deadline: float,
        hwnd: int,
        expected: str,
    ) -> bool:
        observations = max(2, ctx.config.vision.state_confirmation_count)
        for _ in range(observations - 1):
            await self._check_control(ctx, control)
            if monotonic() >= deadline:
                raise TimeoutError(expected)
            await asyncio.sleep(0.01)
            window = ctx.window.find_window()
            if (
                window is None
                or window.hwnd != hwnd
                or not window.is_responding
                or window.is_minimized
            ):
                return False
            try:
                frame = ctx.capture.capture(window.client_rect, window.hwnd)
            except Exception:
                return False
            ctx.metrics.screenshots += 1
            if self._detect_stage(frame.image) != expected:
                return False
        return True

    @staticmethod
    def _detect_stage(image: Any) -> str | None:
        if detect_home_book(image).found:
            return "home_lobby"
        if is_activity_popup(image):
            return "activity_popup"
        if is_announcement(image):
            return "announcement"
        if is_platform_choice(image):
            return "platform_choice"
        if is_login_landing(image):
            return "login_landing"
        return None

    @staticmethod
    def _stage_action(ctx: AutomationContext, stage: str) -> tuple[Point, str]:
        if stage == "platform_choice":
            point = Point(1320, 600) if ctx.config.automation.login_platform == "ios" else Point(600, 600)
            return point, "select_login_platform"
        if stage == "login_landing":
            return Point(960, 540), "enter_game"
        if stage == "announcement":
            return Point(65, 80), "close_announcement"
        if stage == "activity_popup":
            return Point(65, 80), "close_activity_popup"
        raise ValueError(f"unsupported login stage: {stage}")

    async def _click(
        self,
        ctx: AutomationContext,
        deadline: float,
        client_rect,
        reference: Point,
        action: str,
    ) -> None:
        await self._random_delay(
            ctx.config.automation.click_after_detection_ms,
            ctx.config.automation.click_interval_jitter_ms,
            deadline,
        )
        point = CoordinateSpace().to_client(reference, client_rect)
        ctx.input.click(point)
        await ctx.emit(
            "planned_action",
            action=action,
            input="left_click",
            target={"x": point.x, "y": point.y},
        )
        await self._random_delay(
            ctx.config.automation.screenshot_after_click_ms,
            ctx.config.automation.click_interval_jitter_ms,
            deadline,
        )

    @staticmethod
    async def _check_control(ctx: AutomationContext, control: TaskControl) -> None:
        await control.wait_if_paused()
        if control.cancelled.is_set() or ctx.cancellation.cancelled:
            raise _TaskStopped

    @staticmethod
    async def _random_delay(base_ms: int, jitter_ms: int, deadline: float) -> None:
        remaining = deadline - monotonic()
        if remaining <= 0:
            raise TimeoutError("next_screen")
        delay = max(0.01, (base_ms + random.uniform(-jitter_ms, jitter_ms)) / 1000)
        await asyncio.sleep(min(delay, remaining))
