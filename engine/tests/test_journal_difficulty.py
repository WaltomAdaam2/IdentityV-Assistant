import cv2
import numpy as np
import pytest
from pathlib import Path

from core.resources import resource_root
from vision.journal_difficulty import detect_difficulty, detect_s1_floor


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
@pytest.mark.parametrize("floor", ["1", "2", "3"])
def test_s1_uses_selected_pool_and_detected_floor(monkeypatch, tmp_path, kind, floor):
    from app import journal_recognition as service
    from config.loader import parse_config
    from test_journal_matcher import map_image
    path = tmp_path / "b1.png"
    cv2.imwrite(str(path), map_image())
    prepared, records = [], []
    def forbidden(_):
        raise AssertionError("S1 must not use S0 floor labels")
    monkeypatch.setattr(service, "detect_difficulty", forbidden)
    monkeypatch.setattr(service, "detect_s1_floor", lambda _: (floor, {"slot_count": 4}))
    def match(_frame, templates, **kwargs):
        prepared.extend(t.key for t in templates)
        return []
    monkeypatch.setattr(service, "recognize", match)
    pools = {key: [{"key": "wrong", "path": "must-not-open"}] for key in ("hard", "nightmare")}
    pools[kind] = [{"key": "td:1", "floor": f, "path": str(path) if f == floor else "must-not-open"} for f in ("1", "2", "3")]
    def emit(name, **details):
        if name == "journal_recognition_diagnostic":
            records.append(details)
    result = service.run_recognition({"season": "s1", "difficulty": kind,
        "templates_by_difficulty": pools}, parse_config({}), emit,
        capture=lambda *_: frame("nightmare"))
    assert prepared == ["td:1"]
    assert result["difficulty"] == kind
    assert result["floor"] == floor
    assert any(item.get("stage") == "difficulty_selected" and item["mode"] == "manual" for item in records)
    assert any(item.get("stage") == "floor_detected" and item["floor"] == floor for item in records)


def vertical_frame(kind):
    image = np.full((900, 1600, 3), 32, np.uint8)
    patch = cv2.imread(str(Path(__file__).parent / "fixtures/journal" / f"{kind}_vertical.png"))
    image[80:500, 1340:1470] = patch
    return image


@pytest.mark.parametrize("kind", ["hard", "nightmare"])
@pytest.mark.parametrize("size", [(1280, 720), (1600, 900), (1920, 1080)])
def test_november7_vertical_s0_selectors(kind, size):
    detected, details = detect_difficulty(cv2.resize(vertical_frame(kind), size))
    assert detected == kind
    assert details["layout"] == "vertical" and details["floor"] == "1"


@pytest.mark.parametrize("damage", ["missing", "hidden_basement", "hidden_top", "two_highlights"])
def test_damaged_vertical_s0_group_is_not_guessed(damage):
    image = vertical_frame("nightmare")
    if damage == "missing":
        image[:] = 32
    elif damage == "hidden_basement":
        image[344:420, 1374:1428] = 0
    elif damage == "hidden_top":
        image[194:269, 1374:1428] = 0
    else:
        image[194:269, 1374:1428] = image[269:344, 1374:1428]
    with pytest.raises(ValueError):
        detect_difficulty(image)


def synthetic_s1(selected, x=1374, y=190):
    """Synthetic slot/highlight contract, not evidence of actual S1 HUD capture."""
    image = np.full((900, 1600, 3), 32, np.uint8)
    for index in range(4):
        top = y + index * 74
        tile = image[top:top + 74, x:x + 54]
        tile[:] = 108 if index == selected else 38
        color = (20, 20, 20) if index == selected else (160, 160, 160)
        cv2.line(tile, (12, 36), (21, 36), color, 2)
        if index == 0:
            cv2.line(tile, (32, 27), (30, 45), color, 2)
        else:
            cv2.putText(tile, str(index + 1), (26, 45), cv2.FONT_HERSHEY_SIMPLEX, .75, color, 1, cv2.LINE_8)
    return image


@pytest.mark.parametrize("selected", [0, 1, 2])
@pytest.mark.parametrize("size,position", [((1600, 900), (1374, 190)), ((1280, 720), (1130, 260)), ((1920, 1080), (1240, 120))])
def test_synthetic_s1_detects_each_supported_highlight_at_varied_positions(selected, size, position):
    floor, details = detect_s1_floor(cv2.resize(synthetic_s1(selected, *position), size))
    assert floor == str(selected + 1)
    assert details["slot_count"] == 4


def test_synthetic_s1_accepts_three_visible_negative_slots_without_inventing_b4():
    image = synthetic_s1(2)
    image[412:486, 1374:1428] = 32
    floor, details = detect_s1_floor(image)
    assert floor == "3" and details["slot_count"] == 3


@pytest.mark.parametrize("damage", ["b4", "hidden_first", "hidden_middle", "no_highlight", "two_highlights", "two_groups"])
def test_synthetic_s1_rejects_unsupported_or_ambiguous_floor(damage):
    image = synthetic_s1(3 if damage == "b4" else 0)
    if damage == "hidden_first":
        image = synthetic_s1(2)
        image[190:264, 1374:1428] = 32
    elif damage == "hidden_middle":
        image[264:338, 1374:1428] = 0
    elif damage == "no_highlight":
        image[190:264, 1374:1428] = 38
    elif damage == "two_highlights":
        image[264:338, 1374:1428] = synthetic_s1(1)[264:338, 1374:1428]
    elif damage == "two_groups":
        image[:, 1100:1154] = synthetic_s1(1, 1100)[:, 1100:1154]
    with pytest.raises(ValueError):
        detect_s1_floor(image)


@pytest.mark.parametrize("basement", [False, True])
def test_vertical_s0_rejects_other_selected_floors(basement):
    image = np.full((900, 1600, 3), 32, np.uint8)
    for index in range(3 if basement else 2):
        tile = image[190 + index * 74:264 + index * 74, 1374:1428]
        selected = index == (2 if basement else 0)
        tile[:] = 108 if selected else 38
        color = (20, 20, 20) if selected else (160, 160, 160)
        if index == 0:
            cv2.putText(tile, "2", (20, 45), cv2.FONT_HERSHEY_SIMPLEX, .75, color, 1)
        else:
            cv2.line(tile, (30, 27), (28, 45), color, 2)
            if index == 2:
                cv2.line(tile, (10, 36), (19, 36), color, 2)
    with pytest.raises(ValueError, match="S0 recognition requires"):
        detect_difficulty(image)
