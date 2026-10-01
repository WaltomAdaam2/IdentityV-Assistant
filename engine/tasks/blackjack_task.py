from __future__ import annotations

import asyncio
import math
import random
from contextlib import suppress
from time import monotonic, sleep

from app.context import AutomationContext
from core.enums import TaskFinishReason
from core.models import CoordinateSpace, Point
from input.windows_sendinput import WindowsSendInput
from tasks.base import AutomationTask, TaskControl
from vision.blackjack_detector import BlackjackDetector, MARKS
from vision.home_detector import detect_home_book


class _Stopped(Exception):
    pass


class _WindowLost(Exception):
    pass


SURVIVOR_INTERVALS = {
    "right": (10.0, 15.0),
    "left": (5.0, 10.0),
    "e": (1.0, 6.0),
    "q": (5.0, 10.0),
    "r_double": (10.0, 15.0),
    "turn": (10.0, 15.0),
}


def hunter_timeline(rng: random.Random) -> tuple[float, float, float, float]:
    """T and left are measured from right; E follows left, then cooldown."""
    return (
        rng.uniform(3.5, 4.5),
        rng.uniform(5.0, 8.0),
        rng.uniform(5.0, 7.0),
        rng.uniform(10.0, 12.0),
    )


class BlackjackTask(AutomationTask):
    def __init__(self, rng: random.Random | None = None) -> None:
        self.rng = rng or random.Random()
        self.detector = BlackjackDetector()
        self.cycles = 0
        self.role: str | None = None
        self.stage = "starting"
        self._resume_recheck = False
        self._recovering = False

    async def run(self, ctx: AutomationContext, control: TaskControl) -> TaskFinishReason:
        previous_backend = ctx.input.backend
        try:
            await self._capture(ctx, control)
            window = await self._input_window(ctx, control)
            ctx.input.backend = WindowsSendInput(window.hwnd)
            while True:
                await self._cycle(ctx, control)
                self.cycles += 1
                await self._progress(ctx, "cycle_complete")
                if not ctx.config.automation.blackjack_infinite_loop:
                    return TaskFinishReason.COMPLETED
        except _Stopped:
            return TaskFinishReason.USER_STOPPED
        except _WindowLost as error:
            await self._log(ctx, "error", f"Blackjack stopped: {error}")
            return TaskFinishReason.WINDOW_LOST
        except TimeoutError as error:
            await self._log(ctx, "warning", f"Blackjack timed out: {error}")
            return TaskFinishReason.TIMEOUT
        except Exception as error:
            await self._log(ctx, "error", f"Blackjack failed: {error}")
            return TaskFinishReason.ENGINE_ERROR
        finally:
            ctx.input.release_all()
            ctx.input.backend = previous_backend

    async def _cycle(self, ctx: AutomationContext, control: TaskControl) -> None:
        actions = {
            "home": "open_book",
            "chaos": "choose_chaos",
            "blackjack": "choose_blackjack",
            "start_case": "start_case_reconstruction",
            "ready_case": "accept_match",
            "ready_start": "ready_to_start",
            "card_confirm": "accept_card",
            "continue": "continue_after_match",
            "return_lobby": "return_to_lobby",
        }
        last_stage: str | None = None
        acted_stage: str | None = None
        returning = False
        unknown_since: float | None = None
        loading_since: float | None = None
        while True:
            image = await self._capture(ctx, control)
            stage = await self._detect_stage(ctx, control, image)
            if stage is None:
                if unknown_since is None:
                    unknown_since = monotonic()
                if monotonic() - unknown_since >= 300:
                    raise TimeoutError("unknown Blackjack screen for 300 seconds")
                await self._sleep(ctx, control, 0.25)
                continue
            unknown_since = None
            if stage == "loading":
                if loading_since is None:
                    loading_since = monotonic()
                if monotonic() - loading_since >= 300:
                    raise TimeoutError("loading lasted longer than 300 seconds")
            else:
                loading_since = None
            if stage != last_stage:
                await self._progress(ctx, stage)
                acted_stage = None
                last_stage = stage
            if stage == "home" and returning:
                self.role = None
                return
            if stage in ("hunter", "survivor"):
                self.role = stage
                returning = True
                await self._progress(ctx, "playing")
                await self._play(ctx, control)
                last_stage = None
                continue
            if stage in ("continue", "return_lobby"):
                returning = True
            if stage in actions and acted_stage != stage:
                if await self._click(ctx, control, stage, actions[stage]):
                    acted_stage = stage
            await self._sleep(ctx, control, 0.25)

    async def _detect_stage(self, ctx: AutomationContext, control: TaskControl, image) -> str | None:
        priority = (
            "return_lobby", "continue", "card_confirm", "ready_start", "ready_case",
            "start_case", "blackjack", "chaos", "matching", "home",
            "hunter", "survivor", "loading",
        )
        for stage in priority:
            if stage in ("hunter", "survivor"):
                role, _ = self.detector.role(image)
                if role != stage:
                    continue
                await self._sleep(ctx, control, 0.01)
                confirmed, _ = self.detector.role(await self._capture(ctx, control))
                if confirmed == stage:
                    return stage
                continue
            if stage == "home":
                found = detect_home_book(image).found
            else:
                found = self.detector.detect(image, stage).found
            if found and await self._confirmed(ctx, control, image, stage):
                return stage
        return None

    async def _progress(self, ctx: AutomationContext, stage: str) -> None:
        self.stage = stage
        await ctx.emit(
            "task_progress",
            task_id="blackjack",
            stage=stage,
            role=self.role,
            completed_cycles=self.cycles,
            infinite_loop=ctx.config.automation.blackjack_infinite_loop,
        )

    @staticmethod
    async def _log(ctx: AutomationContext, level: str, message: str) -> None:
        await ctx.emit("log", level=level, source="blackjack", message=message, data={"important": True})

    def _window(self, ctx: AutomationContext):
        window = ctx.window.find_window()
        if window is None:
            raise _WindowLost("game window is closed")
        if window.is_minimized or not window.is_foreground:
            for _ in range(3):
                ctx.window.bring_to_front()
                sleep(0.05)
                window = ctx.window.find_window()
                if window is not None and window.is_foreground and not window.is_minimized:
                    break
        if window is None or window.is_minimized or not window.is_foreground:
            raise _WindowLost("could not bring the game window to the foreground")
        return window

    async def _input_window(self, ctx: AutomationContext, control: TaskControl):
        started = monotonic()
        while True:
            try:
                window = self._window(ctx)
                backend = ctx.input.backend
                if isinstance(backend, WindowsSendInput) and backend._expected_hwnd != window.hwnd:
                    ctx.input.release_all()
                    ctx.input.backend = WindowsSendInput(window.hwnd)
                return window
            except _WindowLost:
                if monotonic() - started >= 300:
                    raise
                await self._capture(ctx, control)

    def _capture_once(self, ctx: AutomationContext):
        window = self._window(ctx)
        ctx.memory_guard.protect()
        frame = ctx.capture.capture(window.client_rect, window.hwnd)
        ctx.metrics.screenshots += 1
        return self.detector.normalize(frame.image)

    async def _capture(self, ctx: AutomationContext, control: TaskControl):
        started: float | None = None
        while True:
            await self._check(ctx, control)
            try:
                image = self._capture_once(ctx)
                if self._recovering:
                    await self._log(ctx, "info", "Game window capture recovered")
                    self._recovering = False
                    self._resume_recheck = True
                return image
            except (_WindowLost, RuntimeError, OSError, ValueError) as error:
                if started is None:
                    started = monotonic()
                    self._recovering = True
                    await self._log(ctx, "warning", f"Waiting for game window capture: {error}")
                if monotonic() - started >= 300:
                    await self._log(ctx, "error", "Game window capture unavailable for 300 seconds")
                    raise _WindowLost("game window capture unavailable for 300 seconds") from error
                await self._sleep(ctx, control, 0.25)

    async def _check(self, ctx: AutomationContext, control: TaskControl) -> None:
        if control.cancelled.is_set() or ctx.cancellation.cancelled:
            raise _Stopped
        if not control.resumed.is_set():
            ctx.input.release_all()
            await control.wait_if_paused()
            if control.cancelled.is_set() or ctx.cancellation.cancelled:
                raise _Stopped
            self._resume_recheck = True

    async def _sleep(self, ctx: AutomationContext, control: TaskControl, seconds: float) -> None:
        remaining = max(0.0, seconds)
        while remaining > 0:
            await self._check(ctx, control)
            step = min(remaining, 0.05)
            await asyncio.sleep(step)
            remaining -= step

    async def _confirmed(self, ctx: AutomationContext, control: TaskControl, image, name: str) -> bool:
        await self._check(ctx, control)
        if name == "home":
            if not detect_home_book(image).found:
                return False
            await asyncio.sleep(0.01)
            return detect_home_book(await self._capture(ctx, control)).found
        first = self.detector.detect(image, name)
        if not first.found:
            return False
        await self._sleep(ctx, control, 0.01)
        second = self.detector.detect(await self._capture(ctx, control), name)
        result = second if second.found else None
        if result:
            await ctx.emit(
                "recognition_evidence", task_id="blackjack", stage=name,
                confidence=round(result.confidence, 4), bbox=result.bbox,
            )
        return result is not None

    async def _wait_for(self, ctx: AutomationContext, control: TaskControl, name: str) -> None:
        await self._progress(ctx, f"waiting_{name}")
        active = 0.0
        loading = 0.0
        while active < 300.0:
            await self._check(ctx, control)
            image = await self._capture(ctx, control)
            if await self._confirmed(ctx, control, image, name):
                await self._progress(ctx, name)
                return
            if name != "loading" and await self._confirmed(ctx, control, image, "loading"):
                if loading == 0:
                    await self._progress(ctx, "loading")
                loading += 0.25
                if loading >= 300:
                    raise TimeoutError("loading lasted longer than 300 seconds")
            else:
                loading = 0.0
                active += 0.25
            await self._sleep(ctx, control, 0.25)
        raise TimeoutError(f"waiting for {name} exceeded 300 seconds")

    async def _click(self, ctx: AutomationContext, control: TaskControl, name: str, action: str) -> bool:
        config = ctx.config.automation
        await self._sleep(ctx, control, max(0, config.click_after_detection_ms + self.rng.uniform(-config.click_interval_jitter_ms, config.click_interval_jitter_ms)) / 1000)
        image = await self._capture(ctx, control)
        if not await self._confirmed(ctx, control, image, name):
            return False
        window = await self._input_window(ctx, control)
        reference = self._click_target(name)
        if reference is None:
            raise ValueError(f"no click target for {name}")
        client = CoordinateSpace().to_client(Point(*reference), window.client_rect)
        screen = Point(window.client_rect.left + client.x, window.client_rect.top + client.y)
        actual = ctx.input._jitter_point(screen)
        await self._curved_move(ctx, control, actual)
        await self._input_window(ctx, control)
        ctx.input.mouse_down("left")
        try:
            await self._sleep(ctx, control, max(0.01, (35 + self.rng.uniform(-config.click_duration_jitter_ms, config.click_duration_jitter_ms)) / 1000))
        finally:
            ctx.input.mouse_up("left")
        await ctx.emit("planned_action", action=action, input="foreground_click", target={"x": actual.x, "y": actual.y})
        await self._sleep(ctx, control, config.screenshot_after_click_ms / 1000)
        return True

    def _click_target(self, name: str) -> tuple[int, int] | None:
        return (1800, 760) if name == "home" else MARKS[name].click

    async def _curved_move(self, ctx: AutomationContext, control: TaskControl, target: Point) -> None:
        backend = ctx.input.backend
        if not isinstance(backend, WindowsSendInput):
            raise RuntimeError("foreground input is unavailable")
        start = backend.cursor_position()
        dx, dy = target.x - start.x, target.y - start.y
        distance = math.hypot(dx, dy)
        bend = self.rng.uniform(-0.11, 0.11) * distance
        steps = self.rng.randint(6, 11)
        for index in range(1, steps + 1):
            await self._check(ctx, control)
            t = index / steps
            eased = t * t * (3 - 2 * t)
            arc = 4 * t * (1 - t) * bend / max(distance, 1)
            point = Point(round(start.x + dx * eased - dy * arc), round(start.y + dy * eased + dx * arc))
            backend.move_mouse(point)
            await self._sleep(ctx, control, self.rng.uniform(0.004, 0.012))

    async def _wait_for_role(self, ctx: AutomationContext, control: TaskControl) -> str | None:
        await self._progress(ctx, "waiting_role")
        elapsed = 0.0
        loading = 0.0
        while elapsed < 300:
            image = await self._capture(ctx, control)
            if await self._confirmed(ctx, control, image, "continue"):
                await self._progress(ctx, "continue")
                return None
            role, _ = self.detector.role(image)
            if role:
                await self._sleep(ctx, control, 0.01)
                confirmed, _ = self.detector.role(await self._capture(ctx, control))
                if confirmed == role:
                    self.role = role
                    await self._progress(ctx, "playing")
                    return role
            if await self._confirmed(ctx, control, image, "loading"):
                loading += 0.25
                if loading >= 300:
                    raise TimeoutError("loading lasted longer than 300 seconds")
            else:
                loading = 0.0
                elapsed += 0.25
            await self._sleep(ctx, control, 0.25)
        raise TimeoutError("waiting for a confirmed Blackjack role exceeded 300 seconds")

    async def _before_game_input(self, ctx: AutomationContext, control: TaskControl) -> bool:
        await self._check(ctx, control)
        await self._input_window(ctx, control)
        if self._resume_recheck:
            image = await self._capture(ctx, control)
            role, _ = self.detector.role(image)
            if role != self.role:
                return False
            await self._sleep(ctx, control, 0.01)
            role, _ = self.detector.role(await self._capture(ctx, control))
            if role != self.role:
                return False
            self._resume_recheck = False
        return True

    async def _key(self, ctx: AutomationContext, control: TaskControl, key: str) -> None:
        if await self._before_game_input(ctx, control):
            ctx.input.key_down(key)
            try:
                await self._sleep(ctx, control, self.rng.uniform(0.03, 0.07))
            finally:
                ctx.input.key_up(key)

    async def _mouse_click(self, ctx: AutomationContext, control: TaskControl, button: str) -> None:
        if await self._before_game_input(ctx, control):
            ctx.input.mouse_down(button)
            try:
                await self._sleep(ctx, control, self.rng.uniform(0.03, 0.07))
            finally:
                ctx.input.mouse_up(button)

    async def _turn(self, ctx: AutomationContext, control: TaskControl) -> None:
        if not await self._before_game_input(ctx, control):
            return
        backend = ctx.input.backend
        if not isinstance(backend, WindowsSendInput):
            raise RuntimeError("foreground input is unavailable")
        width = (await self._input_window(ctx, control)).client_rect.width
        angle = self.rng.uniform(60, 150)
        direction = self.rng.choice((-1, 1))
        pixels = max(1, round(width * angle / 360 * ctx.config.automation.camera_turn_scale_percent / 100)) * direction
        steps = self.rng.randint(9, 18)
        sent = 0
        for index in range(1, steps + 1):
            await self._check(ctx, control)
            t = index / steps
            target = round(pixels * (t * t * (3 - 2 * t)))
            backend.move_relative(target - sent, 0)
            sent = target
            await self._sleep(ctx, control, self.rng.uniform(0.005, 0.016))
        await ctx.emit("planned_action", action="turn_camera", input="relative_mouse", degrees=round(angle, 1), direction=direction)

    async def _movement(self, ctx: AutomationContext, control: TaskControl) -> None:
        while True:
            key = self.rng.choice("wasd")
            if await self._before_game_input(ctx, control):
                ctx.input.key_down(key)
                try:
                    await self._sleep(ctx, control, self.rng.uniform(1, 5))
                finally:
                    ctx.input.key_up(key)
            await self._sleep(ctx, control, self.rng.uniform(0.1, 0.6))

    async def _survivor(self, ctx: AutomationContext, control: TaskControl) -> None:
        timers = {name: self.rng.uniform(*bounds) for name, bounds in SURVIVOR_INTERVALS.items()}
        while True:
            delay = min(timers.values())
            await self._sleep(ctx, control, delay)
            for name in timers:
                timers[name] -= delay
            due = [name for name, remaining in timers.items() if remaining <= 0.0001]
            for name in due:
                if name in ("right", "left"):
                    await self._mouse_click(ctx, control, name)
                elif name == "r_double":
                    await self._key(ctx, control, "r")
                    await self._sleep(ctx, control, self.rng.uniform(0.4, 0.6))
                    await self._key(ctx, control, "r")
                elif name == "turn":
                    await self._turn(ctx, control)
                else:
                    await self._key(ctx, control, name)
                timers[name] = self.rng.uniform(*SURVIVOR_INTERVALS[name])

    async def _hunter(self, ctx: AutomationContext, control: TaskControl) -> None:
        last_t = -30.0
        active = 0.0
        while True:
            t_delay, left_delay, e_delay, cooldown = hunter_timeline(self.rng)
            await self._mouse_click(ctx, control, "right")
            if active - last_t >= 30:
                await self._sleep(ctx, control, t_delay)
                active += t_delay
                await self._key(ctx, control, "t")
                last_t = active
                await self._sleep(ctx, control, left_delay - t_delay)
                active += left_delay - t_delay
            else:
                await self._sleep(ctx, control, left_delay)
                active += left_delay
            await self._mouse_click(ctx, control, "left")
            await self._sleep(ctx, control, e_delay)
            active += e_delay
            await self._key(ctx, control, "e")
            await self._sleep(ctx, control, cooldown)
            active += cooldown
            await self._turn(ctx, control)

    async def _play(self, ctx: AutomationContext, control: TaskControl) -> None:
        movement = asyncio.create_task(self._movement(ctx, control))
        actions = asyncio.create_task(self._hunter(ctx, control) if self.role == "hunter" else self._survivor(ctx, control))
        try:
            while True:
                await self._sleep(ctx, control, 5)
                for task in (movement, actions):
                    if task.done() and not task.cancelled():
                        error = task.exception()
                        if error:
                            raise error
                image = await self._capture(ctx, control)
                for result_stage in ("continue", "return_lobby"):
                    if await self._confirmed(ctx, control, image, result_stage):
                        await self._progress(ctx, result_stage)
                        return
                if await self._confirmed(ctx, control, image, "loading"):
                    return
                role, _ = self.detector.role(image)
                if role and role != self.role:
                    await self._sleep(ctx, control, 0.01)
                    confirmed, _ = self.detector.role(await self._capture(ctx, control))
                    if confirmed == role:
                        actions.cancel()
                        await asyncio.gather(actions, return_exceptions=True)
                        ctx.input.release_all()
                        self.role = role
                        await self._progress(ctx, "playing")
                        actions = asyncio.create_task(self._hunter(ctx, control) if role == "hunter" else self._survivor(ctx, control))
        finally:
            movement.cancel()
            actions.cancel()
            await asyncio.gather(movement, actions, return_exceptions=True)
            ctx.input.release_all()
