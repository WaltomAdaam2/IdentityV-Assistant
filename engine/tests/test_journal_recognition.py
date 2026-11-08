import base64
from types import SimpleNamespace

import cv2
import numpy as np
import pytest

from app import journal_recognition as service
from config.loader import parse_config
from core.models import Rect
from test_journal_matcher import map_image


@pytest.mark.parametrize("mode,expected", [("auto", "nightmare"), ("hard", "hard"), ("nightmare", "nightmare")])
def test_s0_difficulty_mode_selects_only_its_pool(monkeypatch, mode, expected):
    detections = []
    events = []
    def detect(frame):
        detections.append(True)
        return "nightmare", {}
    monkeypatch.setattr(service, "detect_difficulty", detect)
    monkeypatch.setattr(service, "recognize", lambda *args, **kwargs: [])
    result = service.run_recognition(
        {"season": "s0", "difficulty": mode, "templates_by_difficulty": {
            "hard": [{"key": "lf:1", "path": "unused-hard.png"}],
            "nightmare": [{"key": "rf:2", "path": "unused-nightmare.png"}],
        }}, parse_config({}), lambda kind, **data: events.append((kind, data)),
        capture=lambda *_: np.zeros((10, 10, 3), np.uint8))
    assert result["difficulty"] == expected
    assert len(detections) == (1 if mode == "auto" else 0)
    indexed = next(data for kind, data in events if data.get("stage") == "indexed")
    assert indexed["template_keys"] == (["lf:1"] if expected == "hard" else ["rf:2"])
    assert result["floor"] == "1"
    updates = [data for kind, data in events if kind == "journal_recognition_progress"]
    assert updates[-1]["difficulty"] == expected
    assert updates[-1]["floor"] == "1"
    assert updates[0]["difficulty"] == (None if mode == "auto" else expected)


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
    assert any(name.startswith("edges_") for name in images)
    assert any(name.startswith("mask_") for name in images)
    assert "template_0" in images
    preview_at = next(i for i, (kind, _) in enumerate(events) if kind == "journal_recognition_preview")
    full_image_at = next(i for i, (kind, _) in enumerate(events) if kind == "journal_recognition_image")
    crop_at = next(i for i, (kind, data) in enumerate(events) if kind == "journal_recognition_preview" and data["preview_kind"] == "region")
    matching_at = next(i for i, (kind, data) in enumerate(events) if kind == "journal_recognition_progress" and data["stage"] == "matching")
    assert events[preview_at][1]["preview_kind"] == "full"
    assert preview_at < full_image_at < crop_at < matching_at
    audits = [data for kind, data in events if kind == "journal_recognition_diagnostic"]
    assert any(data["stage"] == "region_check" for data in audits)
    assert any(data["stage"] == "template_region" and data["attempts"] > 0 and data["closest"] for data in audits)
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


def test_deferred_templates_wait_only_after_capture_and_floor_update(monkeypatch):
    events = []
    monkeypatch.setattr(service, "detect_s1_floor", lambda _: ("3", {}))
    monkeypatch.setattr(service, "recognize", lambda *args, **kwargs: [])
    def templates():
        assert any(kind == "journal_recognition_preview" and data["preview_kind"] == "full" for kind, data in events)
        assert any(kind == "journal_recognition_progress" and data["floor"] == "3" for kind, data in events)
        return {"nightmare": [{"key": "td:1", "floor": "3", "path": "unused.png"}]}
    result = service.run_recognition({"season": "s1", "difficulty": "nightmare", "defer_templates": True},
        parse_config({}), lambda kind, **data: events.append((kind, data)),
        capture=lambda *_: np.zeros((20, 30, 3), np.uint8), load_templates=templates)
    assert result["floor"] == "3"


def test_prepared_helper_loads_current_config_when_requested(monkeypatch, tmp_path):
    import io
    import json
    path = tmp_path / "config.json"
    path.write_text("{}", encoding="utf-8")
    loaded = []
    class RequestInput(io.StringIO):
        def readline(self, *args):
            assert not loaded
            path.write_text('{"updated":true}', encoding="utf-8")
            return super().readline(*args)
    def load_config(actual):
        loaded.append(json.loads(actual.read_text(encoding="utf-8")))
        return parse_config({})
    monkeypatch.setattr(service, "WindowsGraphicsCapture", lambda: None)
    monkeypatch.setattr(service, "load_config", load_config)
    monkeypatch.setattr(service, "run_recognition", lambda *args, **kwargs: {"keys": []})
    monkeypatch.setattr(service.sys, "stdin", RequestInput('{"request_id":"request"}\n'))
    output = io.StringIO()
    monkeypatch.setattr(service.sys, "stdout", output)
    assert service.main(path) == 0
    assert loaded == [{"updated": True}]
    messages = [json.loads(line) for line in output.getvalue().splitlines()]
    assert [message["type"] for message in messages] == ["journal_recognition_ready", "journal_recognition_result"]
    assert messages[0]["request_id"] == ""


def test_offline_replay_uses_exported_pixels_templates_and_feedback(tmp_path):
    import importlib.util
    import json
    from pathlib import Path
    from zipfile import ZIP_DEFLATED, ZipFile
    spec = importlib.util.spec_from_file_location("recognition_replay", Path(__file__).resolve().parents[2] / "scripts/replay_recognition.py")
    replay = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(replay)
    template = map_image()
    path = tmp_path / "template.png"
    path.write_bytes(cv2.imencode(".png", template)[1].tobytes())
    for name, frame in [("matched", template[:, :, :3]), ("no-region", np.zeros((240,320,3),np.uint8))]:
        events = []
        result = service.run_recognition({"templates":[{"key":"lf:1","path":str(path)}]}, parse_config({}),
            lambda kind, **data: events.append((kind,data)), capture=lambda *_: frame)
        steps = [{"stage":"indexed","template_count":1}]
        steps += [data for kind,data in events if kind == "journal_recognition_diagnostic"]
        steps += [{"stage":"completed",**result}]
        report = {"status":"completed","request_id":name,"filters":{},"steps":steps,"feedback":{"verdict":"correct"}}
        bundle = tmp_path / f"{name}.zip"
        with ZipFile(bundle,"w") as output:
            output.writestr(f"recognition/{name}/flow.redacted.json",json.dumps(report))
            for kind,data in events:
                if kind == "journal_recognition_image":
                    output.writestr(f"recognition/{name}/{data['image_kind']}.png",base64.b64decode(data["png"]))
        if name == "no-region":
            # Historical-session bundles may exceed 100 MB in aggregate.
            # Unrelated entries must not be loaded or block this attempt.
            with ZipFile(bundle, "a", compression=ZIP_DEFLATED) as output:
                with output.open("logs/large-history.jsonl", "w") as history:
                    for _ in range(101):
                        history.write(b" " * 1_000_000)
        replayed = replay.replay(bundle)[0]
        assert replayed["status"] == "replayed", replayed
        assert replayed["exact_match"] is True
        assert replayed["feedback"] == {"verdict":"correct"}
        if name == "matched":
            assert replayed["replayed_keys"] == ["lf:1"]
            with ZipFile(bundle,"a") as output:
                output.writestr("../invalid","invalid")
            with pytest.raises(ValueError,match="unsafe"):
                replay.replay(bundle)
