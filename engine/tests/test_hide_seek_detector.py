import cv2

from vision.hide_seek_detector import HideSeekDetector, MARKS, ROOT


SCREENS = {
    "hide_seek": "chaos-menu", "bad_child": "mode-select", "start_game": "mode-select",
    "matching": "matching", "ready_case": "ready-case", "ready_start": "ready-start",
    "f_initial": "f-initial", "r_blocked": "v-ready", "v_ready": "v-ready",
    "v_fixed": "v-fixed", "death_f": "eliminated", "continue": "result-continue",
    "return_lobby": "return-lobby",
}


def reference(name):
    image = cv2.imread(str(ROOT / "reference" / f"{name}.png"))
    assert image is not None
    return image[45:1125, 2:1922]


def test_supplied_screens_are_detected_in_their_regions():
    detector = HideSeekDetector()
    for mark, screen in SCREENS.items():
        assert detector.detect(reference(screen), mark).found, mark
        assert detector.detect(cv2.resize(reference(screen), (1280, 720)), mark).found, mark


def test_result_navigation_does_not_confuse_continue_and_return():
    detector = HideSeekDetector()
    assert not detector.detect(reference("result-continue"), "return_lobby").found
    assert not detector.detect(reference("return-lobby"), "continue").found
    assert MARKS["continue"].search[2] < 1400
    assert MARKS["return_lobby"].search[2] < 1400


def test_death_and_fixed_icons_do_not_match_other_play_states():
    detector = HideSeekDetector()
    for screen in ("f-initial", "v-ready", "v-fixed"):
        assert not detector.detect(reference(screen), "death_f").found
    for screen in ("f-initial", "v-ready", "eliminated"):
        assert not detector.detect(reference(screen), "v_fixed").found
    assert not detector.detect(reference("ready-case"), "matching").found


def test_cross_screen_negatives_allow_only_shared_visible_icons():
    detector = HideSeekDetector()
    allowed = {
        "mode-select": {"bad_child", "start_game"},
        "f-initial": {"f_initial", "living_controls", "living_transform"},
        "v-ready": {"f_initial", "r_blocked", "v_ready", "living_transform"},
        "v-fixed": {"r_blocked", "v_fixed", "living_transform"},
    }
    for mark, screen in SCREENS.items():
        for other in MARKS:
            if other not in allowed.get(screen, {mark}):
                assert not detector.detect(reference(screen), other).found, (screen, other)


def test_living_without_v_requires_both_hud_regions_and_rejects_other_screens():
    detector = HideSeekDetector()
    image = reference("living-no-v")
    assert detector.detect(image, "living_no_v").found
    assert detector.detect(cv2.resize(image, (1280, 720)), "living_no_v").found
    for name in ("living_controls", "living_transform"):
        missing = image.copy()
        left, top, right, bottom = MARKS[name].search
        missing[top:bottom, left:right] = 0
        assert not detector.detect(missing, "living_no_v").found
    for screen in set(SCREENS.values()) - {"f-initial"}:
        assert not detector.detect(reference(screen), "living_no_v").found, screen
