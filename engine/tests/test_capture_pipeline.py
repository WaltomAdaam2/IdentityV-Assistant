import asyncio
from datetime import datetime
from pathlib import Path

import numpy as np

from app.engine import AutomationEngine
from capture.base import Frame
from config.schema import EngineConfig
from core.models import Rect
from window.base import WindowInfo


class StaticWindow:
    def __init__(self, window: WindowInfo | None) -> None:
        self.window = window

    def find_window(self) -> WindowInfo | None:
        return self.window


class StaticCapture:
    backend_name = "mss"
    last_error: str | None = None

    def capture(self, region: Rect | None = None) -> Frame:
        assert region == Rect(10, 20, 30, 40)
        return Frame.from_image(np.zeros((40, 30, 3), dtype=np.uint8), "mss")


class FailingCapture:
    backend_name = "unavailable"
    last_error = "dxcam: failed; mss: failed"

    def capture(self, region: Rect | None = None) -> Frame:
        raise RuntimeError("all capture backends failed")


def target_window(*, minimized: bool = False) -> WindowInfo:
    return WindowInfo(
        hwnd=123,
        title="Identity V",
        class_name="GameWindow",
        process_id=456,
        client_rect=Rect(10, 20, 30, 40),
        dpi_scale=1.25,
        process_name="dwrg.exe",
        is_minimized=minimized,
        is_foreground=True,
        match_reasons=("title", "process"),
    )


def test_capture_emits_correlated_frame_and_window_metadata(tmp_path: Path) -> None:
    engine = AutomationEngine(EngineConfig())
    engine.context.window = StaticWindow(target_window())  # type: ignore[assignment]
    engine.context.capture = StaticCapture()  # type: ignore[assignment]
    events: list[tuple[str, dict[str, object]]] = []

    async def emit(event_type: str, **payload: object) -> None:
        events.append((event_type, payload))

    def save_frame(image: object, captured_at: datetime) -> Path:
        path = tmp_path / "frame.png"
        path.write_bytes(b"png")
        return path

    engine.emit = emit  # type: ignore[method-assign]
    engine._save_debug_frame = save_frame  # type: ignore[method-assign]

    asyncio.run(engine._capture_frame("capture-request"))

    diagnostics = next(payload for event_type, payload in events if event_type == "window_diagnostics")
    frame = next(payload for event_type, payload in events if event_type == "debug_frame")
    assert diagnostics["request_id"] == "capture-request"
    assert diagnostics["capture_backend"] == "mss"
    assert frame["request_id"] == "capture-request"
    assert frame["width"] == 30
    assert frame["height"] == 40
    assert frame["region"] == {"left": 10, "top": 20, "width": 30, "height": 40}
    assert engine.context.metrics.screenshots == 1


def test_capture_failure_reports_backend_diagnostics() -> None:
    engine = AutomationEngine(EngineConfig())
    engine.context.window = StaticWindow(target_window())  # type: ignore[assignment]
    engine.context.capture = FailingCapture()  # type: ignore[assignment]
    events: list[tuple[str, dict[str, object]]] = []

    async def emit(event_type: str, **payload: object) -> None:
        events.append((event_type, payload))

    engine.emit = emit  # type: ignore[method-assign]

    asyncio.run(engine._capture_frame("failed-request"))

    assert [event_type for event_type, _ in events] == ["window_diagnostics", "error"]
    error = events[-1][1]
    assert error["code"] == "CAPTURE_FAILED"
    assert error["request_id"] == "failed-request"
    assert error["details"]["capture_last_error"] == "dxcam: failed; mss: failed"  # type: ignore[index]


def test_template_test_never_falls_back_to_full_desktop() -> None:
    engine = AutomationEngine(EngineConfig())
    engine.context.window = StaticWindow(None)  # type: ignore[assignment]
    engine.context.capture = StaticCapture()  # type: ignore[assignment]
    events: list[tuple[str, dict[str, object]]] = []

    async def emit(event_type: str, **payload: object) -> None:
        events.append((event_type, payload))

    engine.emit = emit  # type: ignore[method-assign]

    asyncio.run(engine._test_template({"template": "test/mock_template.pgm"}, "template-request"))

    assert events[-1][0] == "error"
    assert events[-1][1]["code"] == "WINDOW_NOT_FOUND"
    assert events[-1][1]["request_id"] == "template-request"
    assert engine.context.metrics.screenshots == 0
