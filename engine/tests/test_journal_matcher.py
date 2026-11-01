from threading import Event

import cv2
import numpy as np
import pytest

from vision.journal_matcher import (
    RecognitionCancelled, Region, _bgr, _map_surface, _query, locate_regions, match_region,
    prepare_template, recognize,
)
from vision import journal_matcher


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


def test_scaled_query_reuse_preserves_all_candidate_geometry(monkeypatch):
    frame = _bgr(map_image())
    templates = [prepare_template(str(index), map_image(index % 2)) for index in range(6)]
    cached = recognize(frame, templates)
    original = journal_matcher._match_query

    def uncached(template, query, cancel=None, coarse_weights=None, resize=None, audit=None, fields=None):
        return original(template, query, cancel)

    monkeypatch.setattr(journal_matcher, "_match_query", uncached)
    assert recognize(frame, templates) == cached


@pytest.mark.parametrize("size", [(55, 98), (65, 70)])
def test_tiny_visible_geometry_cannot_use_padding_to_pass_region_size_gate(size):
    # A large proposal can shrink to icon-sized geometry after background is
    # removed. Match quality must apply to those actual pixels, not the padding.
    icon = cv2.resize(_bgr(map_image()), size)
    frame = np.zeros((max(100, size[1] + 20), max(100, size[0] + 20), 3), np.uint8)
    frame[10:10 + size[1], 10:10 + size[0]] = icon
    assert _query(frame, Region(0, 0, frame.shape[1], frame.shape[0])) is None


@pytest.mark.parametrize("width,height,accepted", [(49, 57, False), (63, 90, False), (64, 90, True)])
def test_complex_badge_obeys_final_geometry_size_boundary(width, height, accepted):
    # A complex HUD-like silhouette can satisfy corner/contour checks. It must
    # still meet the size gate; this is original generated artwork, not a map.
    points = np.array([(25, 20), (40, 20), (40, 26), (47, 26), (47, 20), (62, 20),
                       (62, 26), (72, 26), (72, 42), (66, 42), (66, 49), (72, 49),
                       (72, 65), (66, 65), (66, 75), (50, 75), (50, 69), (43, 69),
                       (43, 75), (28, 75), (28, 69), (25, 69), (25, 53), (31, 53),
                       (31, 46), (25, 46)], np.float64)
    points = np.rint((points - [25, 20]) * [(width - 2) / 47, (height - 2) / 55] + 20).astype(np.int32)
    frame = np.zeros((140, 140, 3), np.uint8)
    color = tuple(int(v) for v in cv2.cvtColor(np.uint8([[[113, 59, 96]]]), cv2.COLOR_HSV2BGR)[0, 0])
    cv2.fillPoly(frame, [points], color)
    query = _query(frame, Region(0, 0, 140, 140))
    if accepted:
        assert query is not None
        assert (query[2].width, query[2].height) == (width, height)
    else:
        assert query is None


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


@pytest.mark.parametrize("zoom", [0.5, 1.0, 1.5])
def test_map_on_blue_backdrop_keeps_rooms_but_rejects_legend(zoom):
    image = map_image()
    blue = cv2.cvtColor(np.uint8([[[113, 59, 96]]]), cv2.COLOR_HSV2BGR)[0, 0]
    image[image[:, :, 3] > 0, :3] = blue
    backdrop = cv2.cvtColor(np.uint8([[[107, 85, 62]]]), cv2.COLOR_HSV2BGR)[0, 0]
    partial = cv2.resize(image[20:250, 15:245], None, fx=zoom, fy=zoom)
    frame = np.full((650, 1000, 3), backdrop, np.uint8)
    h, w = partial.shape[:2]
    area = frame[180:180 + h, 550:550 + w]
    area[partial[:, :, 3] > 127] = partial[:, :, :3][partial[:, :, 3] > 127]
    for index, text in enumerate(["DRINKS", "CHESTS", "BOXES", "EVENTS", "EXIT"]):
        cv2.putText(frame, text, (35, 220 + index * 60), cv2.FONT_HERSHEY_SIMPLEX,
                    0.8, (155, 155, 155), 2)
    hits = recognize(frame, [prepare_template("correct", image)])
    assert [hit.key for hit in hits] == ["correct"]
    assert hits[0].region.x >= 550
    legend = frame[185:495, 20:210]
    assert _query(legend, Region(20, 185, 190, 310)) is None
    surface = _map_surface(frame)
    assert not surface[20:150, 300:500].any()
    assert surface[180:180 + h, 550:550 + w].any()
