from types import SimpleNamespace

import cv2
import numpy as np
import pytest

from app import journal_recognition as service
from config.loader import parse_config
from core.models import Rect
from test_journal_matcher import map_image


def test_window_capture_uses_only_wgc_and_preserves_layout(monkeypatch):
    calls = []
    rect = Rect(10, 20, 400, 300)
    window = SimpleNamespace(hwnd=123, client_rect=rect, is_minimized=False, is_responding=True)
    monkeypatch.setattr(service, "WindowsWindowManager", lambda config: SimpleNamespace(find_window=lambda: window))
    class Capture:
        def capture(self, region, hwnd):
            calls.append((region, hwnd))
            return SimpleNamespace(image=np.zeros((300, 400, 3), np.uint8))
    monkeypatch.setattr(service, "WindowsGraphicsCapture", Capture)
    service.capture_window(parse_config({}))
    assert calls == [(rect, 123)]
    window.is_minimized = True
    with pytest.raises(ValueError, match="Restore"):
        service.capture_window(parse_config({}))
    assert len(calls) == 1


def test_one_shot_returns_candidates_and_memory_preview_without_capture_files(tmp_path):
    image = map_image()
    path = tmp_path / "地图.png"
    path.write_bytes(cv2.imencode(".png", image)[1].tobytes())
    events = []
    result = service.run_recognition(
        {"templates": [{"key": "lf:1", "path": str(path)}]}, parse_config({}),
        lambda kind, **data: events.append((kind, data)), capture=lambda config, hwnd: image[:, :, :3])
    assert result["keys"] == ["lf:1"]
    assert result["preview"].startswith("data:image/jpeg;base64,")
    assert {data["stage"] for _, data in events} == {"capturing", "locating", "matching"}
    assert list(tmp_path.iterdir()) == [path]
