import asyncio

import cv2
import numpy as np

from vision.blackjack_detector import BlackjackDetector, MARKS, TEMPLATE_ROOT


SCREEN_NAMES = {
    "chaos": "book-menu",
    "blackjack": "chaos-menu",
    "start_case": "jack-menu",
    "matching": "matching",
    "ready_case": "ready-case",
    "ready_start": "ready-start",
    "card_confirm": "card-confirm",
    "continue": "continue",
    "return_lobby": "return-lobby",
    "loading": "loading",
    "hunter": "hunter",
    "survivor": "survivor",
}


def reference(name: str):
    screenshot = cv2.imread(str(TEMPLATE_ROOT / "reference" / f"{name}.png"))
    assert screenshot is not None
    return screenshot[45:1125, 2:1922]


def test_reference_screens_match_only_their_own_mark() -> None:
    detector = BlackjackDetector()
    for mark, screen in SCREEN_NAMES.items():
        image = reference(screen)
        assert detector.detect(image, mark).found, mark
        for other in MARKS:
            if other != mark:
                assert not detector.detect(image, other).found, (mark, other)


def test_resized_frame_is_normalized_before_matching() -> None:
    detector = BlackjackDetector()
    reduced = cv2.resize(reference("chaos-menu"), (1280, 720))
    assert detector.detect(reduced, "blackjack").found


def test_confirmation_rejects_screen_changed_after_ten_milliseconds() -> None:
    detector = BlackjackDetector()
    image = reference("chaos-menu")
    blank = np.zeros_like(image)
    assert asyncio.run(detector.confirm(image, "blackjack", lambda: blank)) is None
    assert asyncio.run(detector.confirm(image, "blackjack", lambda: image)) is not None
