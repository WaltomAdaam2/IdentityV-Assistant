from threading import Event

import cv2
import numpy as np
import pytest

from vision.journal_matcher import (
    RecognitionCancelled, Region, _bgr, locate_regions, match_region,
    prepare_template, recognize,
)


def map_image(variant: int = 0) -> np.ndarray:
    """Original geometric fixture, not a redistributed game map."""
    image = np.zeros((320, 360, 4), np.uint8)
    color = (95, 100, 105, 255)
    rooms = [(20, 25, 85, 90), (140, 30, 220, 75), (270, 35, 335, 110),
             (35, 165, 110, 240), (170, 150, 240, 225), (265, 235, 330, 290)]
    for index, (left, top, right, bottom) in enumerate(rooms):
        if variant and index in (1, 4):
            left += 15 * variant
            bottom -= 20 * variant
        cv2.rectangle(image, (left, top), (right, bottom), color, -1)
    for start, end in [((55, 60), (300, 60)), ((55, 60), (70, 200)),
                       ((70, 200), (205, 185)), ((205, 185), (300, 265)),
                       ((300, 60), (300, 265))]:
        cv2.line(image, start, end, color, 12)
    return image


def test_transparent_hidden_rgb_does_not_become_map_structure():
    image = map_image()
    altered = image.copy()
    altered[image[:, :, 3] == 0, :3] = (240, 80, 150)
    np.testing.assert_array_equal(prepare_template("map", image).edges,
                                  prepare_template("map", altered).edges)
    half = np.array([[[200, 100, 50, 128]]], np.uint8)
    np.testing.assert_array_equal(_bgr(half), np.array([[[100, 50, 25]]], np.uint8))


@pytest.mark.parametrize("zoom", [0.5, 1.0, 1.75])
def test_partial_exploration_matches_across_scale(zoom):
    image = map_image()
    template = prepare_template("hard:triple:1", image)
    partial = _bgr(image)[20:250, 15:245]
    partial = cv2.resize(partial, None, fx=zoom, fy=zoom, interpolation=cv2.INTER_LINEAR)
    match = match_region(template, partial, Region(0, 0, partial.shape[1], partial.shape[0]))
    assert match is not None
    assert match.key == "hard:triple:1"
    assert match.coverage >= 0.6


def test_whole_window_localization_and_ambiguous_identities():
    panel = _bgr(map_image())
    frame = np.full((600, 960, 3), (35, 40, 45), np.uint8)
    frame[125:445, 540:900] = panel
    cv2.putText(frame, "Game HUD", (40, 65), cv2.FONT_HERSHEY_SIMPLEX, 1, (180, 180, 180), 2)
    regions = locate_regions(frame)
    assert any(r.x <= 560 and r.y <= 150 and r.x + r.width >= 870
               and r.y + r.height >= 415 for r in regions)
    template = prepare_template("hard:lf:1", map_image())
    twin = prepare_template("hard:rf:1", map_image())
    matches = recognize(frame, [template, twin])
    assert {match.key for match in matches} == {"hard:lf:1", "hard:rf:1"}
    assert all(match.region.x > 450 for match in matches)


def test_no_map_and_cancellation_are_not_forced_matches():
    template = prepare_template("one", map_image())
    assert recognize(np.zeros((600, 960, 3), np.uint8), [template]) == []
    cancelled = Event()
    cancelled.set()
    with pytest.raises(RecognitionCancelled):
        recognize(_bgr(map_image()), [template], cancelled)


@pytest.mark.parametrize("size", [(65, 65), (80, 45)])
def test_plain_hud_rectangle_is_not_a_map(size):
    frame = np.zeros((600, 960, 3), np.uint8)
    cv2.rectangle(frame, (600, 200), (600 + size[0], 200 + size[1]), (100, 100, 100), -1)
    assert recognize(frame, [prepare_template("map", map_image())]) == []


@pytest.mark.parametrize("kind", ["outline", "two_buttons", "label"])
def test_hud_borders_and_button_text_are_not_map_structure(kind):
    frame = np.zeros((600, 960, 3), np.uint8)
    cv2.rectangle(frame, (500, 200), (580, 260), (100, 100, 100), 5 if kind == "outline" else -1)
    if kind == "two_buttons":
        cv2.rectangle(frame, (600, 200), (680, 245), (100, 100, 100), -1)
    elif kind == "label":
        cv2.putText(frame, "OK", (512, 237), cv2.FONT_HERSHEY_SIMPLEX, 0.7, (255, 255, 255), 2)
    assert recognize(frame, [prepare_template("map", map_image())]) == []


def test_bad_inputs_fail_before_geometry_processing():
    with pytest.raises(ValueError):
        prepare_template("bad", np.zeros((10, 10, 2), np.uint8))
    with pytest.raises(ValueError):
        prepare_template("empty", np.zeros((40, 40, 4), np.uint8))


def test_small_map_is_not_displaced_by_large_invalid_hud_panels():
    frame = np.full((1080, 1920, 3), 190, np.uint8)
    for row in range(3):
        for col in range(5):
            x, y = 20 + 360 * col, 20 + 260 * row
            cv2.rectangle(frame, (x, y), (x + 280, y + 180), (100, 100, 100), 5)
    frame[880:1040, 1700:1880] = cv2.resize(_bgr(map_image()), (180, 160))
    assert [match.key for match in recognize(frame, [prepare_template("map", map_image())])] == ["map"]
