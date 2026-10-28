from __future__ import annotations

from pathlib import Path
from typing import Any
from core.resources import read_template, resource_root

from vision.blackjack_detector import BlackjackDetector, Detection, ScreenMark


ROOT = resource_root() / "assets" / "templates"

COMMON = {
    "diary": ScreenMark("diary", (620, 610, 960, 770), (775, 690)),
    "hunter_choice": ScreenMark("hunter_choice", (1100, 150, 1610, 300), (1300, 510)),
    "start_case": ScreenMark("start_case", (750, 900, 1200, 1050), (960, 985)),
    "queue_warning": ScreenMark("queue_warning", (380, 330, 1500, 520), (1190, 625)),
    "normal_matching": ScreenMark("normal_matching", (0, 260, 300, 380)),
    "ready_case": ScreenMark("ready_case", (740, 780, 1200, 1010), (960, 900)),
    "ready_start": ScreenMark("ready_start", (1360, 870, 1840, 1050), (1600, 975)),
    "surrender": ScreenMark("surrender", (1210, 785, 1640, 970), (1385, 900)),
    "result_first": ScreenMark("result_first", (0, 885, 410, 1070), (960, 985)),
    "result_second": ScreenMark("result_second", (0, 250, 235, 420), (960, 985)),
    "return_lobby": ScreenMark("return_lobby", (740, 870, 1210, 1050), (960, 985)),
}
RANK = {
    "rank_entry": ScreenMark("rank_entry", (590, 210, 980, 400), (780, 310)),
    "traditional_rank": ScreenMark("traditional_rank", (330, 360, 790, 540), (565, 455)),
    "hunter_choice": ScreenMark("hunter_choice", (1100, 150, 1610, 300), (1300, 510)),
    "start_case": ScreenMark("start_case", (750, 900, 1200, 1050), (960, 985)),
    "rank_matching": ScreenMark("rank_matching", (0, 260, 300, 380)),
}


class HunterDetector:
    def __init__(self, ranked: bool = False) -> None:
        self.ranked = ranked
        self._templates: dict[tuple[str, str], Any] = {}
        self._role_detector = BlackjackDetector()

    normalize = staticmethod(BlackjackDetector.normalize)

    def _mark(self, name: str) -> tuple[ScreenMark, Path]:
        if self.ranked and name in RANK:
            return RANK[name], ROOT / "hunter_rank"
        return COMMON[name], ROOT / "hunter_match"

    def detect(self, image: Any, name: str) -> Detection:
        import cv2

        if name == "playing":
            return self._role_detector.detect(image, "hunter")
        if name == "start_case" and self.detect(image, "queue_warning").found:
            return Detection(False, 0.0, None)
        if name == "result_first" and (
            self.detect(image, "result_second").found
            or self.detect(image, "return_lobby").found
        ):
            return Detection(False, 0.0, None)
        if name in ("normal_matching", "rank_matching") and self.detect(image, "ready_case").found:
            return Detection(False, 0.0, None)
        mark, root = self._mark(name)
        key = (str(root), mark.template)
        if key not in self._templates:
            template = read_template(root / f"{mark.template}.png")
            if template is None:
                raise FileNotFoundError(root / f"{mark.template}.png")
            self._templates[key] = template
        template = self._templates[key]
        frame = self.normalize(image)
        left, top, right, bottom = mark.search
        region = cv2.cvtColor(frame[top:bottom, left:right], cv2.COLOR_BGR2GRAY)
        if region.shape[0] < template.shape[0] or region.shape[1] < template.shape[1]:
            return Detection(False, 0.0, None)
        result = cv2.matchTemplate(region, template, cv2.TM_CCOEFF_NORMED)
        _, confidence, _, location = cv2.minMaxLoc(result)
        box = (left + location[0], top + location[1], template.shape[1], template.shape[0])
        return Detection(confidence >= mark.threshold, float(confidence), box)

    def click_point(self, name: str) -> tuple[int, int]:
        mark, _ = self._mark(name)
        if mark.click is None:
            raise ValueError(f"No click target for {name}")
        return mark.click
