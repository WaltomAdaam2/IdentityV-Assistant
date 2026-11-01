import base64
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
    assert {data["stage"] for kind, data in events if kind == "journal_recognition_progress"} == {"capturing", "locating", "matching"}
    images = {data["image_kind"]: cv2.imdecode(np.frombuffer(base64.b64decode(data["png"]), np.uint8), cv2.IMREAD_COLOR)
              for kind, data in events if kind == "journal_recognition_image"}
    np.testing.assert_array_equal(images["full"], image[:, :, :3])
    region = result["matches"][0]["region"]
    x, y, w, h = (region[key] for key in ("x", "y", "width", "height"))
    np.testing.assert_array_equal(images["region"], image[y:y+h, x:x+w, :3])
    assert list(tmp_path.iterdir()) == [path]


def test_no_match_still_returns_full_capture_for_diagnostics(tmp_path):
    frame = np.zeros((200, 300, 3), np.uint8)
    events = []
    result = service.run_recognition(
        {"templates": [{"key": "lf:1", "path": str(tmp_path / "unused.png")}]}, parse_config({}),
        lambda kind, **data: events.append((kind, data)), capture=lambda config, hwnd: frame)
    assert result["keys"] == []
    assert [data["image_kind"] for kind, data in events if kind == "journal_recognition_image"] == ["full"]
    assert list(tmp_path.iterdir()) == []
