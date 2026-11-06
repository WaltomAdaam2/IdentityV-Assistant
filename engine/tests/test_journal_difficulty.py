import cv2
import numpy as np
import pytest

from core.resources import resource_root
from vision.journal_difficulty import detect_difficulty


def frame(kind, offset=0):
    image = np.full((900, 1600, 3), 32, np.uint8)
    reference = cv2.imread(str(resource_root() / f"assets/templates/journal/{kind}_floors.png"))
    x = (1292 if kind == "hard" else 1174) + offset
    image[44:85, x:x + reference.shape[1]] = reference
    return image


@pytest.mark.parametrize("kind", ["hard", "nightmare"])
@pytest.mark.parametrize("size", [(1600, 900), (1280, 720), (1920, 1080)])
def test_visible_floor_labels_survive_window_scaling(kind, size):
    assert detect_difficulty(cv2.resize(frame(kind), size))[0] == kind


def test_hard_selector_can_move_left_without_becoming_nightmare():
    assert detect_difficulty(frame("hard", -59))[0] == "hard"


@pytest.mark.parametrize("damage", ["missing", "two_hidden", "basement_obscured"])
def test_incomplete_or_obscured_floor_selector_is_rejected(damage):
    image = frame("nightmare")
    if damage == "missing":
        image[:] = 32
    elif damage == "two_hidden":
        image[44:85, 1305:1355] = 32
    else:
        image[44:85, 1174:1210] = 0
    with pytest.raises(ValueError, match="floor labels"):
        detect_difficulty(image)


def test_capture_selects_detected_pool_before_loading_templates(monkeypatch, tmp_path):
    from app import journal_recognition as service
    from config.loader import parse_config
    from test_journal_matcher import map_image
    path = tmp_path / "map.png"
    cv2.imwrite(str(path), map_image())
    prepared = []
    monkeypatch.setattr(service, "detect_difficulty", lambda _: ("hard", {}))
    def match(_frame, templates, **kwargs):
        prepared.extend(t.key for t in templates)
        return []
    monkeypatch.setattr(service, "recognize", match)
    result = service.run_recognition({"templates_by_difficulty": {
        "hard": [{"key": "lf:1", "path": str(path)}],
        "nightmare": [{"key": "wrong", "path": "must-not-open"}],
    }}, parse_config({}), lambda *args, **kwargs: None, capture=lambda *_: frame("hard"))
    assert prepared == ["lf:1"]
    assert result["difficulty"] == "hard"


@pytest.mark.parametrize("kind", ["hard", "nightmare"])
def test_s1_uses_selected_pool_without_s0_floor_detection(monkeypatch, tmp_path, kind):
    from app import journal_recognition as service
    from config.loader import parse_config
    from test_journal_matcher import map_image
    path = tmp_path / "b1.png"
    cv2.imwrite(str(path), map_image())
    prepared, records = [], []
    def forbidden(_):
        raise AssertionError("S1 must not use S0 floor labels")
    monkeypatch.setattr(service, "detect_difficulty", forbidden)
    def match(_frame, templates, **kwargs):
        prepared.extend(t.key for t in templates)
        return []
    monkeypatch.setattr(service, "recognize", match)
    pools = {key: [{"key": "wrong", "path": "must-not-open"}] for key in ("hard", "nightmare")}
    pools[kind] = [{"key": "td:1", "path": str(path)}]
    def emit(name, **details):
        if name == "journal_recognition_diagnostic":
            records.append(details)
    result = service.run_recognition({"season": "s1", "difficulty": kind,
        "templates_by_difficulty": pools}, parse_config({}), emit,
        capture=lambda *_: frame("nightmare"))
    assert prepared == ["td:1"]
    assert result["difficulty"] == kind
    assert any(item.get("stage") == "difficulty_selected" and item["mode"] == "manual" for item in records)
