from __future__ import annotations

import asyncio
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any


REFERENCE_SIZE = (1920, 1080)
TEMPLATE_ROOT = Path(__file__).resolve().parents[2] / "assets" / "templates" / "blackjack"


@dataclass(frozen=True)
class ScreenMark:
    template: str
    search: tuple[int, int, int, int]
    click: tuple[int, int] | None = None
    threshold: float = 0.78


MARKS = {
    "chaos": ScreenMark("chaos_chars", (1010, 395, 1225, 535), (1115, 460)),
    "blackjack": ScreenMark("jack_char", (460, 245, 665, 370), (555, 310)),
    "start_case": ScreenMark("case_chars", (815, 920, 1120, 1045), (970, 985)),
    "matching": ScreenMark("match_mode", (0, 265, 250, 350)),
    "ready_case": ScreenMark("ready_case", (770, 820, 1160, 965), (965, 900)),
    "ready_start": ScreenMark("ready_start", (1410, 900, 1820, 1045), (1600, 975)),
    "card_confirm": ScreenMark("confirm", (1060, 880, 1330, 1030), (1185, 950)),
    "continue": ScreenMark("continue", (800, 905, 1115, 1050), (960, 985)),
    "return_lobby": ScreenMark("return_lobby", (780, 905, 1140, 1055), (960, 985)),
    "loading": ScreenMark("loading", (600, 220, 1330, 855), threshold=0.67),
    "hunter": ScreenMark("hunter_icon", (0, 930, 140, 1080), threshold=0.72),
    "survivor": ScreenMark("survivor_icon", (0, 930, 140, 1080), threshold=0.72),
}


@dataclass(frozen=True)
class Detection:
    found: bool
    confidence: float
    bbox: tuple[int, int, int, int] | None


class BlackjackDetector:
    def __init__(self) -> None:
        self._templates: dict[str, Any] = {}

    @staticmethod
    def normalize(image: Any) -> Any:
        import cv2

        if image is None or len(image.shape) < 2:
            raise ValueError("a captured image is required")
        if (image.shape[1], image.shape[0]) == REFERENCE_SIZE:
            return image
        return cv2.resize(image, REFERENCE_SIZE, interpolation=cv2.INTER_AREA)

    def _template(self, name: str) -> Any:
        import cv2

        if name not in self._templates:
            image = cv2.imread(str(TEMPLATE_ROOT / f"{name}.png"), cv2.IMREAD_GRAYSCALE)
            if image is None:
                raise FileNotFoundError(f"Missing blackjack template: {name}")
            self._templates[name] = image
        return self._templates[name]

    def detect(self, image: Any, name: str) -> Detection:
        import cv2

        mark = MARKS[name]
        frame = self.normalize(image)
        if name == "matching" and self.detect(frame, "ready_case").found:
            return Detection(False, 0.0, None)
        left, top, right, bottom = mark.search
        region = cv2.cvtColor(frame[top:bottom, left:right], cv2.COLOR_BGR2GRAY)
        template = self._template(mark.template)
        if region.shape[0] < template.shape[0] or region.shape[1] < template.shape[1]:
            return Detection(False, 0.0, None)
        result = cv2.matchTemplate(region, template, cv2.TM_CCOEFF_NORMED)
        _, confidence, _, location = cv2.minMaxLoc(result)
        box = (left + location[0], top + location[1], template.shape[1], template.shape[0])
        return Detection(confidence >= mark.threshold, float(confidence), box)

    def role(self, image: Any) -> tuple[str | None, dict[str, float]]:
        hunter = self.detect(image, "hunter")
        survivor = self.detect(image, "survivor")
        scores = {"hunter": hunter.confidence, "survivor": survivor.confidence}
        if hunter.found and hunter.confidence > survivor.confidence + 0.07:
            return "hunter", scores
        if survivor.found and survivor.confidence > hunter.confidence + 0.07:
            return "survivor", scores
        return None, scores

    async def confirm(
        self, image: Any, name: str, capture_again: Callable[[], Any]
    ) -> Detection | None:
        first = self.detect(image, name)
        if not first.found:
            return None
        await asyncio.sleep(0.01)
        second = self.detect(capture_again(), name)
        return second if second.found else None
