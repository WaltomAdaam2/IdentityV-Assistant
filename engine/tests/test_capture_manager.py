import numpy as np
import pytest

from capture.base import Frame
from capture.manager import CaptureManager
from config.schema import CaptureConfig
from core.models import Rect


class FailingCapture:
    def capture(self, region: Rect | None = None, hwnd: int | None = None) -> Frame:
        raise RuntimeError("device unavailable")


class FailingValueCapture:
    def capture(self, region: Rect | None = None, hwnd: int | None = None) -> Frame:
        raise ValueError("region is outside the selected output")


class WorkingCapture:
    def capture(self, region: Rect | None = None, hwnd: int | None = None) -> Frame:
        assert region == Rect(10, 20, 30, 40)
        return Frame.from_image(np.zeros((40, 30, 3), dtype=np.uint8), "mss")


def test_auto_backend_falls_back_to_mss_after_dxcam_capture_error(monkeypatch: pytest.MonkeyPatch) -> None:
    manager = CaptureManager(CaptureConfig(backend="auto"))
    manager.backend = FailingCapture()
    manager.backend_name = "dxcam"
    monkeypatch.setattr(manager, "_create_backend", lambda name: WorkingCapture())

    frame = manager.capture(Rect(10, 20, 30, 40))

    assert frame.source == "mss"
    assert manager.backend_name == "mss"
    assert manager.last_error == "dxcam: device unavailable"


def test_capture_rejects_empty_region() -> None:
    manager = CaptureManager(CaptureConfig())
    with pytest.raises(ValueError, match="positive width and height"):
        manager.capture(Rect(0, 0, 0, 100))


def test_auto_backend_falls_back_for_backend_specific_exceptions(monkeypatch: pytest.MonkeyPatch) -> None:
    manager = CaptureManager(CaptureConfig(backend="auto"))
    manager.backend = FailingValueCapture()
    manager.backend_name = "dxcam"
    monkeypatch.setattr(manager, "_create_backend", lambda name: WorkingCapture())

    frame = manager.capture(Rect(10, 20, 30, 40))

    assert frame.source == "mss"
    assert manager.backend_name == "mss"
    assert manager.last_error == "dxcam: region is outside the selected output"


def test_backend_starts_uninitialized() -> None:
    manager = CaptureManager(CaptureConfig())

    assert manager.backend_name == "not_initialized"
