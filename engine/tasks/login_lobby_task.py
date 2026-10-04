from __future__ import annotations

import asyncio
import random
from dataclasses import dataclass
from time import monotonic
from typing import Any

from app.context import AutomationContext
from core.enums import TaskFinishReason
from core.models import CoordinateSpace, Point, Rect
from tasks.base import AutomationTask, TaskControl
from vision.blackjack_detector import BlackjackDetector
from vision.home_detector import detect_home_book
from vision.login_detector import is_activity_popup, is_announcement, is_login_landing, is_platform_choice


class _TaskStopped(Exception):
    pass


@dataclass
class _ActiveDeadline:
    expires_at: float


class LoginLobbyTask(AutomationTask):
    """Select a login platform, dismiss startup pages, and wait for the home lobby."""

    async def run(self, ctx: AutomationContext, control: TaskControl) -> TaskFinishReason:
        deadline = _ActiveDeadline(monotonic() + ctx.config.automation.login_timeout_seconds)
        try:
            await self._wait_for_window(ctx, control, deadline)
            await self._random_delay(
                ctx,
                control,
                350,
                ctx.config.automation.click_interval_jitter_ms,
                deadline,
            )
            last_action_stage = ""
            last_action_at = 0.0
            while True:
                stage, window, deadline = await self._wait_for_confirmed_stage(ctx, control, deadline)
                if stage == "home_lobby":
                    break
                if stage == last_action_stage and monotonic() - last_action_at < 2.0:
                    await self._random_delay(ctx, control, 250, 50, deadline)
                    continue
                point, action = self._stage_action(ctx, stage)
                if await self._click(ctx, control, deadline, stage, point, action):
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

    async def _wait_for_window(
        self,
        ctx: AutomationContext,
        control: TaskControl,
        deadline: _ActiveDeadline,
    ):
        while True:
            await self._check_control(ctx, control, deadline)
            if monotonic() >= deadline.expires_at:
                break
            window = ctx.window.find_window()
            if window is not None and window.is_responding:
                return window
            await self._random_delay(
                ctx,
                control,
                500,
                ctx.config.automation.click_interval_jitter_ms,
                deadline,
            )
        raise TimeoutError("game_window")

    async def _wait_for_confirmed_stage(
        self,
        ctx: AutomationContext,
        control: TaskControl,
        deadline: _ActiveDeadline,
    ):
        loading_detector = BlackjackDetector()
        loading_elapsed = 0.0
        while True:
            await self._check_control(ctx, control, deadline)
            if monotonic() >= deadline.expires_at:
                break
            window = ctx.window.find_window()
            if window is not None and window.is_responding and not window.is_minimized:
                ctx.memory_guard.protect()
                try:
                    frame = ctx.capture.capture(window.client_rect, window.hwnd)
                except Exception:
                    frame = None
                if frame is not None:
                    ctx.metrics.screenshots += 1
                    if loading_detector.detect(frame.image, "loading").found:
                        await self._random_delay(ctx, control, 10, 0, deadline)
                        again = ctx.capture.capture(window.client_rect, window.hwnd)
                        ctx.metrics.screenshots += 1
                        if loading_detector.detect(again.image, "loading").found:
                            loading_elapsed += 0.25
                            if loading_elapsed >= 300:
                                raise TimeoutError("loading animation")
                            await self._random_delay(
                                ctx,
                                control,
                                250,
                                0,
                                deadline,
                                freeze_deadline=True,
                            )
                            continue
                    loading_elapsed = 0.0
                    stage = self._detect_stage(frame.image)
                    if stage and await self._confirm_stage(
                        ctx, control, deadline, window.hwnd, stage
                    ):
                        await ctx.emit("login_stage", stage=stage, found=True)
                        current = ctx.window.find_window()
                        if current is not None:
                            return stage, current, deadline
            await self._random_delay(
                ctx,
                control,
                250,
                ctx.config.automation.click_interval_jitter_ms,
                deadline,
            )
        raise TimeoutError("a recognizable login screen")

    async def _confirm_stage(
        self,
        ctx: AutomationContext,
        control: TaskControl,
        deadline: _ActiveDeadline,
        hwnd: int,
        expected: str,
    ) -> bool:
        observations = max(2, ctx.config.vision.state_confirmation_count)
        for _ in range(observations - 1):
            await self._check_control(ctx, control, deadline)
            if monotonic() >= deadline.expires_at:
                raise TimeoutError(expected)
            await self._random_delay(ctx, control, 10, 0, deadline)
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
        control: TaskControl,
        deadline: _ActiveDeadline,
        stage: str,
        reference: Point,
        action: str,
    ) -> bool:
        await self._random_delay(
            ctx,
            control,
            ctx.config.automation.click_after_detection_ms,
            ctx.config.automation.click_interval_jitter_ms,
            deadline,
        )
        await self._check_control(ctx, control, deadline)
        window = ctx.window.find_window()
        if window is None or not window.is_responding or window.is_minimized:
            return False
        try:
            frame = ctx.capture.capture(window.client_rect, window.hwnd)
        except Exception:
            return False
        ctx.metrics.screenshots += 1
        if self._detect_stage(frame.image) != stage:
            return False
        if not await self._confirm_stage(ctx, control, deadline, window.hwnd, stage):
            return False
        await self._check_control(ctx, control, deadline)
        point = CoordinateSpace().to_client(reference, window.client_rect)
        radius_x = max(1, round(20 * window.client_rect.width / 1920))
        radius_y = max(1, round(20 * window.client_rect.height / 1080))
        ctx.input.click(point, bounds=Rect(point.x - radius_x, point.y - radius_y, 2 * radius_x + 1, 2 * radius_y + 1))
        await ctx.emit(
            "planned_action",
            action=action,
            input="left_click",
            target={"x": point.x, "y": point.y},
        )
        await self._random_delay(
            ctx,
            control,
            ctx.config.automation.screenshot_after_click_ms,
            ctx.config.automation.click_interval_jitter_ms,
            deadline,
        )
        return True

    @staticmethod
    async def _check_control(
        ctx: AutomationContext,
        control: TaskControl,
        deadline: _ActiveDeadline,
    ) -> None:
        if not control.resumed.is_set():
            ctx.input.release_all()
            paused_at = monotonic()
            await control.wait_if_paused()
            deadline.expires_at += monotonic() - paused_at
        if control.cancelled.is_set() or ctx.cancellation.cancelled:
            raise _TaskStopped

    @staticmethod
    async def _random_delay(
        ctx: AutomationContext,
        control: TaskControl,
        base_ms: int,
        jitter_ms: int,
        deadline: _ActiveDeadline,
        *,
        freeze_deadline: bool = False,
    ) -> None:
        delay = max(0.01, (base_ms + random.uniform(-jitter_ms, jitter_ms)) / 1000)
        remaining_delay = delay
        while remaining_delay > 0:
            await LoginLobbyTask._check_control(ctx, control, deadline)
            deadline_remaining = deadline.expires_at - monotonic()
            if deadline_remaining <= 0:
                raise TimeoutError("next_screen")
            step = min(remaining_delay, deadline_remaining, 0.01)
            before = monotonic()
            await asyncio.sleep(step)
            elapsed = monotonic() - before
            remaining_delay -= elapsed
            if freeze_deadline:
                deadline.expires_at += elapsed
