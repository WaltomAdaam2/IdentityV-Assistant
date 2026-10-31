import asyncio
from datetime import datetime, timezone, timedelta
from types import SimpleNamespace
from unittest.mock import patch

import numpy as np
import pytest

from app.engine import AutomationEngine
from config.schema import EngineConfig
from core.models import Rect
from runtime.task_feedback import TaskFeedback
from runtime.failure_capture import save_failure_image
from capture.base import Frame
from capture.manager import CaptureManager
from config.schema import CaptureConfig
from tasks.base import TaskControl
from core.enums import TaskFinishReason


def test_active_stage_time_excludes_pauses_and_resets_for_new_task():
    feedback = TaskFeedback()
    control = SimpleNamespace(paused_seconds=0)
    with patch("runtime.task_feedback.monotonic", return_value=10):
        feedback.start("login_lobby", control)
    with patch("runtime.task_feedback.monotonic", return_value=15):
        feedback.update({"stage": "matching"})
    control.paused_seconds = 4
    with patch("runtime.task_feedback.monotonic", return_value=22):
        assert feedback.snapshot()["task_elapsed_seconds"] == 8
        assert feedback.snapshot()["stage_elapsed_seconds"] == 3
        feedback.start("blackjack", SimpleNamespace(paused_seconds=0))
        assert feedback.snapshot()["task_elapsed_seconds"] == 0
    feedback.clear()
    assert feedback.snapshot() == {}


@pytest.mark.parametrize("condition,expected", [("ready", True), ("missing", False), ("unresponsive", False), ("capture_failed", False)])
def test_preflight_checks_capture_without_saving_or_sending_input(condition, expected):
    async def exercise():
        engine = AutomationEngine(EngineConfig())
        events = []
        async def send(payload): events.append(payload)
        engine.writer = SimpleNamespace(send=send)
        window = SimpleNamespace(is_responding=condition != "unresponsive", is_minimized=False,
                                 is_foreground=False, client_rect=Rect(0, 0, 1280, 720), hwnd=1)
        engine.context.window = SimpleNamespace(find_window=lambda: None if condition == "missing" else window)
        engine._window_health_payload = lambda window: {"found": window is not None}
        captures = []
        def capture(*args):
            captures.append(True)
            if condition == "capture_failed": raise RuntimeError("fixture")
            return SimpleNamespace(width=1280, height=720)
        engine.context.capture = SimpleNamespace(capture=capture)
        assert await engine._preflight("fixture") is expected
        assert events[-1]["type"] == "task_preflight"
        assert events[-1]["request_id"] == "fixture"
        if expected:
            assert events[-1]["warnings"] == ["resolution_differs", "window_not_foreground"]
        if condition in {"missing", "unresponsive"}:
            assert not captures
    asyncio.run(exercise())


def test_failure_retention_preserves_manual_files_and_limits_own_images(tmp_path):
    manual = tmp_path / "manual.png"
    manual.write_bytes(b"untouched")
    first = None
    for index in range(22):
        path = save_failure_image(np.zeros((2, 2, 3), dtype=np.uint8),
                                  datetime(2026, 1, 1, tzinfo=timezone.utc) + timedelta(seconds=index), tmp_path)
        first = first or path
    assert len(list(tmp_path.glob("failure-*.png"))) == 20
    assert not first.exists()
    assert manual.read_bytes() == b"untouched"


def test_failure_capture_opt_out_does_not_read_or_save_frame():
    async def exercise():
        engine = AutomationEngine(EngineConfig(debug={"save_failure_screenshots": False}))
        engine.context.capture = None
        await engine._record_failure(None)
    asyncio.run(exercise())


def test_failure_images_also_respect_total_byte_limit(tmp_path):
    for index in range(5):
        with (tmp_path / f"failure-20260101T00000{index}000000Z-00000000.png").open("wb") as file:
            file.truncate(20 * 1024 * 1024)
    save_failure_image(np.zeros((2, 2, 3), dtype=np.uint8), datetime.now(timezone.utc), tmp_path)
    assert sum(path.stat().st_size for path in tmp_path.iterdir()) <= 100 * 1024 * 1024
    assert not (tmp_path / "failure-20260101T000000000000Z-00000000.png").exists()


def test_failure_image_write_error_is_reported_without_raising():
    async def exercise():
        engine = AutomationEngine(EngineConfig())
        events = []
        async def send(payload): events.append(payload)
        engine.writer = SimpleNamespace(send=send)
        engine.context.capture.last_frame = Frame.from_image(np.zeros((2, 2, 3), dtype=np.uint8), "wgc")
        engine.context.capture.last_frame_at = datetime.now(timezone.utc)
        with patch("app.engine.save_failure_image", side_effect=OSError("disk full")):
            await engine._record_failure(TaskFinishReason.TIMEOUT)
        assert events[-1]["saved"] is False
        assert events[-1]["reason"] == "save_failed"
    asyncio.run(exercise())


@pytest.mark.parametrize("command", [{"type":"start_task","task":"login_lobby"},
                                     {"type":"start_tasks","tasks":["login_lobby","hide_seek"]}])
def test_failed_preflight_does_not_create_runner_or_queue(command):
    async def exercise():
        engine = AutomationEngine(EngineConfig())
        events = []
        async def send(payload): events.append(payload)
        engine.writer = SimpleNamespace(send=send)
        engine.context.window = SimpleNamespace(find_window=lambda: None)
        engine._window_health_payload = lambda window: {"found": False}
        await engine._handle_command(command)
        assert engine._task_runner is None
        assert not engine._task_queue
        assert events[-1]["type"] == "task_preflight" and events[-1]["ok"] is False
    asyncio.run(exercise())


def test_only_window_surface_capture_is_retained_for_automatic_failure_images():
    capture = CaptureManager(CaptureConfig())
    capture.retain_last_frame = True
    image = np.zeros((2, 2, 3), dtype=np.uint8)
    capture._remember(Frame.from_image(image, "wgc"), 42)
    assert capture.last_frame is not None
    image[:] = 255
    assert not capture.last_frame.image.any()
    for source in ["mss", "dxcam"]:
        capture._remember(Frame.from_image(image, source), 42)
        assert capture.last_frame is None


def test_stop_during_failure_image_save_does_not_pause_or_deadlock():
    async def exercise():
        engine = AutomationEngine(EngineConfig())
        events = []
        async def send(payload): events.append(payload)
        engine.writer = SimpleNamespace(send=send)
        engine.context.window = SimpleNamespace(find_window=lambda: None)
        engine.context.input = SimpleNamespace(release_all=lambda: None)
        engine._task_control = TaskControl()
        entered, finish_save = asyncio.Event(), asyncio.Event()
        async def save(reason):
            entered.set()
            await finish_save.wait()
        engine._record_failure = save
        class FailedTask:
            async def run(self, context, control): return TaskFinishReason.TIMEOUT
        engine._task_runner = asyncio.create_task(engine._run_task(FailedTask()))
        await entered.wait()
        stop = asyncio.create_task(engine._stop_task(TaskFinishReason.USER_STOPPED))
        await asyncio.sleep(0)
        finish_save.set()
        await asyncio.wait_for(stop, timeout=1)
        assert not any(item["type"] == "task_paused" for item in events)
        assert next(item for item in events if item["type"] == "task_finished")["reason"] == "user_stopped"
    asyncio.run(exercise())
