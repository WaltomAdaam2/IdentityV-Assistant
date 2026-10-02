from __future__ import annotations

from pathlib import Path
from typing import Any

from vision.blackjack_detector import BlackjackDetector, Detection, ScreenMark


ROOT = Path(__file__).resolve().parents[2] / "assets" / "templates" / "hide_seek"


MARKS = {
    "hide_seek": ScreenMark("hide_seek", (180, 585, 510, 780), (350, 700)),
    "bad_child": ScreenMark("bad_child", (75, 250, 390, 455), (245, 385)),
    "start_game": ScreenMark("start_game", (750, 825, 1190, 1015), (970, 925)),
    "matching": ScreenMark("matching", (0, 270, 310, 420)),
    "ready_case": ScreenMark("ready_case", (735, 795, 1200, 995), (960, 900)),
    "ready_start": ScreenMark("ready_start", (1360, 865, 1840, 1050), (1600, 975)),
    "f_initial": ScreenMark("f_initial", (1760, 900, 1920, 1080), threshold=0.74),
    "r_blocked": ScreenMark("r_blocked", (1760, 790, 1920, 1010), threshold=0.74),
    "v_ready": ScreenMark("v_ready", (70, 905, 330, 1080), threshold=0.74),
    "v_fixed": ScreenMark("v_fixed", (70, 905, 330, 1080), threshold=0.74),
    "death_f": ScreenMark("death_f", (1760, 895, 1920, 1080), threshold=0.74),
    "continue": ScreenMark("continue", (760, 890, 1190, 1060), (960, 985), threshold=0.94),
    "return_lobby": ScreenMark("return_lobby", (760, 890, 1190, 1060), (960, 985), threshold=0.94),
}


class HideSeekDetector:
    """Restricted-region template detector for the Hide and Seek workflow."""

    normalize = staticmethod(BlackjackDetector.normalize)

    def __init__(self) -> None:
        self._templates: dict[str, Any] = {}

    def _template(self, name: str) -> Any:
        import cv2

        if name not in self._templates:
            template = cv2.imread(str(ROOT / f"{name}.png"), cv2.IMREAD_GRAYSCALE)
            if template is None:
                raise FileNotFoundError(ROOT / f"{name}.png")
            self._templates[name] = template
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
        bbox = (left + location[0], top + location[1], template.shape[1], template.shape[0])
        return Detection(confidence >= mark.threshold, float(confidence), bbox)

    @staticmethod
    def click_point(name: str) -> tuple[int, int]:
        point = MARKS[name].click
        if point is None:
            raise ValueError(f"No click target for {name}")
        return point
