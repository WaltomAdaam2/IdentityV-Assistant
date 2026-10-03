from __future__ import annotations

import asyncio
import random
from collections.abc import Callable
from time import monotonic

from app.context import AutomationContext
from core.enums import TaskFinishReason
from input.windows_sendinput import WindowsSendInput
from tasks.base import TaskControl
from tasks.blackjack_task import BlackjackTask, _Stopped, _WindowLost
from vision.blackjack_detector import BlackjackDetector
from vision.hide_seek_detector import HideSeekDetector
from vision.home_detector import detect_home_book


GAME_STAGES = {"f_initial", "r_blocked", "v_ready", "v_fixed", "living_no_v", "death_f"}


def movement_timing(rng: random.Random) -> tuple[float, float]:
    return rng.uniform(13, 17), rng.uniform(1, 5)


def camera_timing(rng: random.Random) -> tuple[float, float]:
    return rng.uniform(2.5, 3.5), rng.uniform(2.5, 3.5)


class HideSeekTask(BlackjackTask):
    task_id = "hide_seek"

    def __init__(self, rng: random.Random | None = None, clock: Callable[[], float] = monotonic) -> None:
        super().__init__(rng)
        self.detector = HideSeekDetector()
        self.shared_detector = BlackjackDetector()
        self.role = "bad_child"
        self._clock = clock
        self._reset_round()

    def _reset_round(self) -> None:
        self._fixed_seen = False
        self._escape_active = False
        self._escape_step = 0
        self._next_alt_at = None

    def _now(self, control: TaskControl | None) -> float:
        return self._clock() - (control.paused_seconds if control is not None else 0.0)

    def _infinite(self, ctx: AutomationContext) -> bool:
        return bool(getattr(ctx.config.automation, "hide_seek_infinite_loop", False))

    async def run(self, ctx: AutomationContext, control: TaskControl) -> TaskFinishReason:
        previous_backend = ctx.input.backend
        try:
            await self._capture(ctx, control)
            ctx.input.backend = WindowsSendInput((await self._input_window(ctx, control)).hwnd)
            while True:
                await self._cycle(ctx, control)
                self.cycles += 1
                await self._progress(ctx, "cycle_complete")
                if not self._infinite(ctx):
                    return TaskFinishReason.COMPLETED
        except _Stopped:
            return TaskFinishReason.USER_STOPPED
        except _WindowLost as error:
            await self._log(ctx, "error", f"Hide and Seek stopped: {error}")
            return TaskFinishReason.WINDOW_LOST
        except TimeoutError as error:
            await self._log(ctx, "warning", f"Hide and Seek timed out: {error}")
            return TaskFinishReason.TIMEOUT
        except Exception as error:
            await self._log(ctx, "error", f"Hide and Seek failed: {error}")
            return TaskFinishReason.ENGINE_ERROR
        finally:
            ctx.input.release_all()
            ctx.input.backend = previous_backend

    async def _log(self, ctx: AutomationContext, level: str, message: str) -> None:
        await ctx.emit("log", level=level, source=self.task_id, message=message, data={"important": True})

    async def _progress(self, ctx: AutomationContext, stage: str) -> None:
        self.stage = stage
        await ctx.emit(
            "task_progress", task_id=self.task_id, stage=stage, role=self.role,
            completed_cycles=self.cycles, infinite_loop=self._infinite(ctx),
        )

    def _click_target(self, name: str) -> tuple[int, int] | None:
        if name == "home":
            return (1800, 760)
        if name == "chaos":
            from vision.blackjack_detector import MARKS

            return MARKS[name].click
        return self.detector.click_point(name)

    async def _confirmed(self, ctx: AutomationContext, control: TaskControl, image, name: str) -> bool:
        await self._check(ctx, control)
        if name == "home":
            detect = lambda frame: detect_home_book(frame)
        elif name in ("chaos", "loading"):
            detect = lambda frame: self.shared_detector.detect(frame, name)
        else:
            detect = lambda frame: self.detector.detect(frame, name)
        if not detect(image).found:
            return False
        await self._sleep(ctx, control, 0.01)
        second = detect(await self._capture(ctx, control))
        if second.found:
            await ctx.emit(
                "recognition_evidence", task_id=self.task_id, stage=name,
                confidence=round(getattr(second, "confidence", 1.0), 4),
                bbox=getattr(second, "bbox", None),
            )
        return second.found

    async def _detect_stage(self, ctx: AutomationContext, control: TaskControl, image) -> str | None:
        for name in (
            "return_lobby", "continue", "death_f", "ready_case", "ready_start", "loading",
            "v_fixed", "v_ready", "living_no_v", "r_blocked", "f_initial", "start_game",
            "bad_child", "hide_seek", "chaos", "matching", "home",
        ):
            if name == "living_no_v" and not self._fixed_seen:
                continue
            if await self._confirmed(ctx, control, image, name):
                return name
        return None

    async def _cycle(self, ctx: AutomationContext, control: TaskControl) -> None:
        self._reset_round()
        actions = {
            "home": "open_book", "chaos": "choose_chaos", "hide_seek": "choose_hide_seek",
            "ready_case": "accept_match", "ready_start": "ready_to_start",
            "continue": "continue_after_elimination", "return_lobby": "return_to_lobby",
        }
        last_stage = None
        acted_stage = None
        returning = False
        unknown_since = None
        loading_since = None
        while True:
            image = await self._capture(ctx, control)
            stage = await self._detect_stage(ctx, control, image)
            if stage is None:
                ctx.input.release_all()
                if unknown_since is None:
                    unknown_since = self._now(control)
                if self._now(control) - unknown_since >= 300:
                    raise TimeoutError("unknown Hide and Seek screen for 300 seconds")
                await self._sleep(ctx, control, 0.25)
                continue
            unknown_since = None
            if stage == "v_fixed":
                self._fixed_seen = True
            elif stage == "living_no_v" and self._fixed_seen:
                self._escape_active = True
            elif stage not in GAME_STAGES or stage == "death_f":
                self._reset_round()
            if stage == "loading":
                if loading_since is None:
                    loading_since = self._now(control)
            else:
                loading_since = None
            if loading_since is not None and self._now(control) - loading_since >= 300:
                raise TimeoutError("loading lasted longer than 300 seconds")
            if stage != last_stage:
                await self._progress(ctx, stage)
                last_stage, acted_stage = stage, None
            if stage == "home" and returning:
                return
            if stage == "death_f":
                returning = True
                ctx.input.release_all()
                if acted_stage != stage:
                    await self._key(ctx, control, "esc")
                    acted_stage = stage
            elif stage in GAME_STAGES:
                returning = True
                await self._play(ctx, control)
                last_stage = None
                continue
            if stage in ("continue", "return_lobby"):
                returning = True
            if stage in ("bad_child", "start_game") and acted_stage != stage:
                # Both labels share this screen; explicitly select Bad Child before starting.
                if await self._click(ctx, control, "bad_child", "choose_bad_child"):
                    if await self._click(ctx, control, "start_game", "start_hide_seek"):
                        acted_stage = stage
            elif stage in actions and acted_stage != stage:
                if await self._click(ctx, control, stage, actions[stage]):
                    acted_stage = stage
            await self._sleep(ctx, control, 0.25)

    async def _before_game_input(self, ctx: AutomationContext, control: TaskControl) -> bool:
        await self._check(ctx, control)
        await self._input_window(ctx, control)
        if self._resume_recheck:
            image = await self._capture(ctx, control)
            stage = await self._detect_stage(ctx, control, image)
            if stage != self.stage or stage not in GAME_STAGES:
                return False
            self._resume_recheck = False
        return True

    async def _hold(self, ctx: AutomationContext, control: TaskControl, seconds: float) -> None:
        if not await self._before_game_input(ctx, control):
            return
        key = self.rng.choice("wasd")
        ctx.input.key_down(key)
        try:
            await self._sleep(ctx, control, seconds)
        finally:
            ctx.input.key_up(key)

    async def _prepare_hiding(self, ctx: AutomationContext, control: TaskControl, stage: str) -> None:
        if stage == "f_initial":
            await self._hold(ctx, control, self.rng.uniform(1, 3))
            await self._key(ctx, control, "f")
        elif stage == "v_ready":
            image = await self._capture(ctx, control)
            if await self._confirmed(ctx, control, image, "r_blocked"):
                await self._hold(ctx, control, self.rng.uniform(5, 8))
                await self._key(ctx, control, "v")

    async def _movement(self, ctx: AutomationContext, control: TaskControl) -> None:
        while True:
            duration, cooldown = movement_timing(self.rng)
            await self._hold(ctx, control, duration)
            await self._sleep(ctx, control, cooldown)

    async def _camera(self, ctx: AutomationContext, control: TaskControl) -> None:
        while True:
            if not await self._before_game_input(ctx, control):
                return
            duration, cooldown = camera_timing(self.rng)
            direction = self.rng.choice((-1, 1))
            steps = max(1, round(duration / 0.05))
            window = await self._input_window(ctx, control)
            scale = getattr(ctx.config.automation, "camera_turn_scale_percent", 100) / 100
            total = direction * max(1, round(window.client_rect.width * 0.025 * scale))
            sent = 0
            for index in range(1, steps + 1):
                await self._check(ctx, control)
                if self._resume_recheck:
                    return
                progress = index / steps
                target = round(total * progress * progress * (3 - 2 * progress))
                if target != sent:
                    ctx.input.backend.move_relative(target - sent, 0)
                sent = target
                await self._sleep(ctx, control, duration / steps)
            await self._sleep(ctx, control, cooldown)

    async def _fixed_actions(self, ctx: AutomationContext, control: TaskControl) -> None:
        async def delayed_movement() -> None:
            await self._sleep(ctx, control, 5)
            await self._movement(ctx, control)

        movement = asyncio.create_task(delayed_movement())
        camera = asyncio.create_task(self._camera(ctx, control))
        try:
            await asyncio.gather(movement, camera)
        finally:
            movement.cancel()
            camera.cancel()
            await asyncio.gather(movement, camera, return_exceptions=True)
            ctx.input.release_all()

    async def _escape_actions(self, ctx: AutomationContext, control: TaskControl) -> None:
        while self._escape_step < 3:
            if not await self._before_game_input(ctx, control):
                return
            key = ("f", "t", "g")[self._escape_step]
            ctx.input.key_down(key)
            self._escape_step += 1
            try:
                await self._sleep(ctx, control, 0.04)
            finally:
                ctx.input.key_up(key)
        while True:
            if self._next_alt_at is None:
                self._next_alt_at = self._now(control) + self.rng.uniform(7, 13)
            await self._sleep(ctx, control, max(0, self._next_alt_at - self._now(control)))
            if not await self._before_game_input(ctx, control):
                return
            ctx.input.key_down("alt")
            try:
                ctx.input.key_down("1")
                try:
                    await self._sleep(ctx, control, 0.04)
                finally:
                    ctx.input.key_up("1")
            finally:
                ctx.input.key_up("alt")
            self._next_alt_at = None

    async def _play(self, ctx: AutomationContext, control: TaskControl) -> None:
        action = None
        escape = None
        last_stage = None
        action_stage = None
        try:
            while True:
                image = await self._capture(ctx, control)
                stage = await self._detect_stage(ctx, control, image)
                if stage not in GAME_STAGES:
                    if stage is not None:
                        self._reset_round()
                    return
                if stage == "death_f":
                    self._reset_round()
                    return
                if stage == "v_fixed":
                    self._fixed_seen = True
                if stage == "living_no_v" and self._fixed_seen:
                    self._escape_active = True
                next_action_stage = "v_fixed" if self._fixed_seen else stage
                if stage != last_stage:
                    await self._progress(ctx, stage)
                    last_stage = stage
                if next_action_stage != action_stage or self._resume_recheck:
                    pending_actions = [pending for pending in (action, escape) if pending is not None]
                    for pending in pending_actions:
                        pending.cancel()
                    await asyncio.gather(*pending_actions, return_exceptions=True)
                    escape = None
                    ctx.input.release_all()
                    self._resume_recheck = False
                    action_stage = next_action_stage
                    if self._fixed_seen:
                        action = asyncio.create_task(self._fixed_actions(ctx, control))
                    else:
                        action = asyncio.create_task(self._prepare_hiding(ctx, control, stage))
                if self._escape_active and escape is None:
                    escape = asyncio.create_task(self._escape_actions(ctx, control))
                for pending in (action, escape):
                    if pending is not None and pending.done() and not pending.cancelled():
                        error = pending.exception()
                        if error:
                            raise error
                await self._sleep(ctx, control, 0.25)
        finally:
            pending_actions = [pending for pending in (action, escape) if pending is not None]
            for pending in pending_actions:
                pending.cancel()
            await asyncio.gather(*pending_actions, return_exceptions=True)
            ctx.input.release_all()
