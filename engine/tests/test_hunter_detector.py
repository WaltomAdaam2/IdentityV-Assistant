import cv2

from vision.hunter_detector import COMMON, RANK, ROOT, HunterDetector


NORMAL_SCREENS = {
    "diary": "book-menu",
    "hunter_choice": "role-select",
    "start_case": "start-case",
    "queue_warning": "queue-warning",
    "normal_matching": "matching",
    "ready_case": "ready-case",
    "ready_start": "ready-start",
    "surrender": "surrender-menu",
    "result_first": "result-first",
    "result_second": "result-second",
    "return_lobby": "return-lobby",
}
RANK_SCREENS = {
    "rank_entry": "book-menu",
    "traditional_rank": "rank-mode",
    "hunter_choice": "role-select",
    "start_case": "start-case",
    "rank_matching": "matching",
}


def reference(mode: str, screen: str):
    screenshot = cv2.imread(str(ROOT / mode / "reference" / f"{screen}.png"))
    assert screenshot is not None
    return screenshot[45:1125, 2:1922]


def test_normal_match_templates_have_no_cross_screen_hits() -> None:
    detector = HunterDetector()
    for mark, screen in NORMAL_SCREENS.items():
        image = reference("hunter_match", screen)
        assert detector.detect(image, mark).found, mark
        for other in COMMON:
            if other != mark:
                assert not detector.detect(image, other).found, (mark, other)


def test_rank_entry_templates_have_no_cross_screen_hits() -> None:
    detector = HunterDetector(ranked=True)
    for mark, screen in RANK_SCREENS.items():
        image = reference("hunter_rank", screen)
        assert detector.detect(image, mark).found, mark
        for other in RANK:
            if other != mark:
                assert not detector.detect(image, other).found, (mark, other)


def test_rank_and_normal_matching_labels_do_not_overlap() -> None:
    normal = HunterDetector()
    ranked = HunterDetector(ranked=True)
    assert not ranked.detect(reference("hunter_match", "matching"), "rank_matching").found
    assert not normal.detect(reference("hunter_rank", "matching"), "normal_matching").found
