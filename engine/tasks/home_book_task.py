from __future__ import annotations

import asyncio

from app.context import AutomationContext
from core.enums import TaskFinishReason
from tasks.base import AutomationTask, TaskControl
from vision.home_detector import detect_home_book, home_book_click_point


class HomeBookTask(AutomationTask):
    """Recognize the home lobby from ten book-icon pixels and click the book."""

    async def run(self, ctx: AutomationContext, control: TaskControl) -> TaskFinishReason:
        await control.wait_if_paused()
        if control.cancelled.is_set() or ctx.cancellation.cancelled:
            return TaskFinishReason.USER_STOPPED

        window = ctx.window.find_window()
        if window is None:
            await ctx.emit(
                "log",
                level="warning",
                source="home_book",
                message="Game window is not open",
                data={"important": True},
            )
            return TaskFinishReason.WINDOW_LOST
        if not window.is_responding:
            await ctx.emit(
                "log",
                level="warning",
                source="home_book",
                message="Game window is not responding",
                data={"important": True},
            )
            return TaskFinishReason.WINDOW_LOST
        if not ctx.window.prepare_background_mode():
            return TaskFinishReason.WINDOW_LOST

        await asyncio.sleep(0.35)
        window = ctx.window.find_window()
        if window is None or not window.is_responding:
            return TaskFinishReason.WINDOW_LOST
        if window.is_minimized:
            return TaskFinishReason.WINDOW_LOST

        memory_percent = ctx.memory_guard.protect()
        if memory_percent is not None and memory_percent > ctx.memory_guard.threshold_percent:
            await ctx.emit(
                "log",
                level="warning",
                source="memory",
                message=f"System memory usage reached {memory_percent:.1f}%; unused caches were released",
                data={"important": True, "memory_percent": memory_percent},
            )

        frame = ctx.capture.capture(window.client_rect, window.hwnd)
        ctx.metrics.screenshots += 1
        detection = detect_home_book(frame.image)
        await ctx.emit(
            "home_detection",
            found=detection.found,
            matched_pixels=detection.matches,
            required_pixels=detection.required,
            total_pixels=detection.total,
        )
        if not detection.found:
            await ctx.emit(
                "log",
                level="warning",
                source="home_book",
                message=f"Home lobby was not recognized ({detection.matches}/10 pixels matched)",
                data={"important": True},
            )
            return TaskFinishReason.TIMEOUT

        point = home_book_click_point(window.client_rect.width, window.client_rect.height)
        ctx.input.click(point, "left")
        await ctx.emit(
            "planned_action",
            action="click_home_book",
            input="left_click",
            target={"x": point.x, "y": point.y, "hwnd": window.hwnd},
            dry_run=ctx.input.dry_run,
        )
        await ctx.emit(
            "log",
            level="info",
            source="home_book",
            message=f"Home lobby recognized ({detection.matches}/10 pixels matched); book click sent",
            data={"important": True},
        )
        return TaskFinishReason.COMPLETED
