from __future__ import annotations

import asyncio
import random
import re
from dataclasses import dataclass
from time import monotonic

from app.context import AutomationContext
from core.enums import TaskFinishReason
from input.windows_sendinput import WindowsSendInput
from tasks.base import TaskControl
from tasks.blackjack_task import BlackjackTask, _Stopped, _WindowLost
from vision.blackjack_detector import BlackjackDetector
from vision.home_detector import detect_home_book
from vision.hunter_detector import HunterDetector
from vision.ocr import RapidOcrEngine


ACTION_INTERVALS = {
    "right": (25.0, 35.0),
    "left": (5.0, 30.0),
    "t": (40.0, 50.0),
    "space": (5.0, 30.0),
    "q": (5.0, 60.0),
}


def parse_game_clock(text: str) -> int | None:
    match = re.search(r"(?<!\d)(\d{1,2})\s*[:：]\s*([0-5]\d)(?!\d)", text)
    return int(match[1]) * 60 + int(match[2]) if match else None


@dataclass
class SurrenderClock:
    started_at: float
    last_valid_at: float

    @classmethod
    def start(cls, now: float) -> SurrenderClock:
        return cls(now, now)

    def due(self, now: float, displayed_seconds: int | None) -> bool:
        if displayed_seconds is not None:
            self.last_valid_at = now
            return displayed_seconds >= 360
        return now - self.started_at >= 360 and now - self.last_valid_at >= 30


class HunterMatchTask(BlackjackTask):
    task_id = "hunter_match"
    ranked = False

    def __init__(self, rng: random.Random | None = None) -> None:
        super().__init__(rng)
        self.detector = HunterDetector(ranked=self.ranked)
        self.loading_detector = BlackjackDetector()
        self.ocr = RapidOcrEngine()
        self._ocr_warning_logged = False

    def _infinite(self, ctx: AutomationContext) -> bool:
        setting = "hunter_rank_infinite_loop" if self.ranked else "hunter_match_infinite_loop"
        return bool(getattr(ctx.config.automation, setting))

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
            await self._log(ctx, "error", f"{self.task_id} stopped: {error}")
            return TaskFinishReason.WINDOW_LOST
        except TimeoutError as error:
            await self._log(ctx, "warning", f"{self.task_id} timed out: {error}")
            return TaskFinishReason.TIMEOUT
        except Exception as error:
            await self._log(ctx, "error", f"{self.task_id} failed: {error}")
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
        return (1800, 760) if name == "home" else self.detector.click_point(name)

    async def _confirmed(self, ctx: AutomationContext, control: TaskControl, image, name: str) -> bool:
        if name == "home":
            if not detect_home_book(image).found:
                return False
            await self._sleep(ctx, control, 0.01)
            return detect_home_book(await self._capture(ctx, control)).found
        if name == "loading":
            detect = lambda frame: self.loading_detector.detect(frame, "loading")
        else:
            detect = lambda frame: self.detector.detect(frame, name)
        if not detect(image).found:
            return False
        await self._sleep(ctx, control, 0.01)
        result = detect(await self._capture(ctx, control))
        if result.found:
            await ctx.emit(
                "recognition_evidence", task_id=self.task_id, stage=name,
                confidence=round(result.confidence, 4), bbox=result.bbox,
            )
        return result.found

    async def _detect_stage(self, ctx: AutomationContext, control: TaskControl, image) -> str | None:
        priorities = [
            "return_lobby", "result_second", "result_first", "queue_warning",
            "surrender", "ready_case", "ready_start", "start_case", "hunter_choice",
        ]
        priorities += ["rank_matching", "traditional_rank", "rank_entry"] if self.ranked else ["normal_matching", "diary"]
        priorities += ["home", "playing", "loading"]
        for name in priorities:
            if await self._confirmed(ctx, control, image, name):
                return name
        return None

    async def _cycle(self, ctx: AutomationContext, control: TaskControl) -> None:
        actions = {
            "home": "open_book",
            "hunter_choice": "choose_hunter",
            "start_case": "start_case_reconstruction",
            "queue_warning": "decline_queue_switch",
            "ready_case": "accept_match",
            "ready_start": "ready_to_start",
            "result_first": "continue_first_result",
            "result_second": "continue_second_result",
            "return_lobby": "return_to_lobby",
        }
        if self.ranked:
            actions.update(rank_entry="choose_rank", traditional_rank="choose_traditional_rank")
        else:
            actions["diary"] = "choose_diary"
        last_stage: str | None = None
        acted_stage: str | None = None
        returning = False
        unknown_since: float | None = None
        loading_since: float | None = None
        queue_retry_at = 0.0
        surrender_retry_at = 0.0
        while True:
            image = await self._capture(ctx, control)
            stage = await self._detect_stage(ctx, control, image)
            if stage is None:
                if unknown_since is None:
                    unknown_since = monotonic()
                if monotonic() - unknown_since >= 300:
                    raise TimeoutError("unknown hunter screen for 300 seconds")
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
            if stage == "playing":
                self.role = "hunter"
                returning = True
                await self._progress(ctx, "playing")
                await self._play(ctx, control)
                last_stage = None
                continue
            if stage in ("result_first", "result_second", "return_lobby"):
                returning = True
            if stage == "queue_warning":
                queue_retry_at = monotonic() + 30
            if stage == "start_case" and monotonic() < queue_retry_at:
                await self._sleep(ctx, control, 0.25)
                continue
            if stage == "surrender":
                returning = True
                if monotonic() >= surrender_retry_at:
                    if await self._click(ctx, control, stage, "surrender"):
                        surrender_retry_at = monotonic() + 15
                await self._sleep(ctx, control, 0.25)
                continue
            if stage in actions and acted_stage != stage:
                if await self._click(ctx, control, stage, actions[stage]):
                    acted_stage = stage
            await self._sleep(ctx, control, 0.25)

    async def _before_game_input(self, ctx: AutomationContext, control: TaskControl) -> bool:
        await self._check(ctx, control)
        await self._input_window(ctx, control)
        if self._resume_recheck:
            image = await self._capture(ctx, control)
            if not await self._confirmed(ctx, control, image, "playing"):
                return False
            self._resume_recheck = False
        return True

    async def _movement(self, ctx: AutomationContext, control: TaskControl) -> None:
        while True:
            key = self.rng.choice("wasd")
            if await self._before_game_input(ctx, control):
                ctx.input.key_down(key)
                try:
                    await self._sleep(ctx, control, self.rng.uniform(5, 15))
                finally:
                    ctx.input.key_up(key)
            await self._sleep(ctx, control, self.rng.uniform(0.1, 0.6))

    async def _combat(self, ctx: AutomationContext, control: TaskControl) -> None:
        timers = {name: self.rng.uniform(*bounds) for name, bounds in ACTION_INTERVALS.items()}
        while True:
            delay = min(timers.values())
            await self._sleep(ctx, control, delay)
            for name in timers:
                timers[name] -= delay
            due = [name for name, remaining in timers.items() if remaining <= 0.0001]
            for name in sorted(due, key=lambda item: item != "e"):
                if name in ("right", "left"):
                    await self._mouse_click(ctx, control, name)
                    if name == "left":
                        timers["e"] = self.rng.uniform(0.3, 3.0)
                else:
                    await self._key(ctx, control, name)
                if name == "e":
                    del timers[name]
                else:
                    timers[name] = self.rng.uniform(*ACTION_INTERVALS[name])

    def _displayed_seconds(self, image) -> int | None:
        import cv2

        gray = cv2.cvtColor(image[20:105, 80:240], cv2.COLOR_BGR2GRAY)
        enhanced = cv2.createCLAHE(clipLimit=4, tileGridSize=(4, 4)).apply(gray)
        region = cv2.resize(enhanced, None, fx=5, fy=5, interpolation=cv2.INTER_CUBIC)
        for result in self.ocr.read_text(region):
            seconds = parse_game_clock(result.text)
            if seconds is not None:
                return seconds
        return None

    async def _play(self, ctx: AutomationContext, control: TaskControl) -> None:
        clock = SurrenderClock.start(monotonic())
        movement = asyncio.create_task(self._movement(ctx, control))
        actions = asyncio.create_task(self._combat(ctx, control))
        surrendering = False
        retry_at = 0.0
        next_clock_check = 0.0
        try:
            while True:
                await self._sleep(ctx, control, 1)
                for task in (movement, actions):
                    if task.done() and not task.cancelled() and task.exception():
                        raise task.exception()
                image = await self._capture(ctx, control)
                for name in ("return_lobby", "result_second", "result_first"):
                    if await self._confirmed(ctx, control, image, name):
                        await self._progress(ctx, name)
                        return
                if await self._confirmed(ctx, control, image, "loading"):
                    return
                if not surrendering and not await self._confirmed(ctx, control, image, "playing"):
                    return
                if not surrendering:
                    displayed = None
                    if monotonic() >= next_clock_check:
                        next_clock_check = monotonic() + 5
                        try:
                            displayed = self._displayed_seconds(image)
                        except RuntimeError as error:
                            if not self._ocr_warning_logged:
                                await self._log(ctx, "warning", f"OCR unavailable; using local timer: {error}")
                                self._ocr_warning_logged = True
                    if clock.due(monotonic(), displayed):
                        surrendering = True
                        movement.cancel()
                        actions.cancel()
                        await asyncio.gather(movement, actions, return_exceptions=True)
                        ctx.input.release_all()
                        await self._progress(ctx, "surrendering")
                if surrendering and monotonic() >= retry_at:
                    if await self._confirmed(ctx, control, image, "surrender"):
                        await self._click(ctx, control, "surrender", "surrender")
                    else:
                        await self._key(ctx, control, "esc")
                    retry_at = monotonic() + 15
        finally:
            movement.cancel()
            actions.cancel()
            await asyncio.gather(movement, actions, return_exceptions=True)
            ctx.input.release_all()


class HunterRankTask(HunterMatchTask):
    task_id = "hunter_rank"
    ranked = True
