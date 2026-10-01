"""Rebuild bounded UI templates from the supplied 1924x1127 screenshots."""

from pathlib import Path

import cv2


ROOT = Path(__file__).resolve().parents[1] / "assets" / "templates"
# Coordinates are in the game client area after removing the window title bar.
SPECS = {
    "hunter_match": {
        "diary": ("book-menu", (726, 653, 842, 705)),
        "hunter_choice": ("role-select", (1377, 191, 1499, 244)),
        "start_case": ("start-case", (922, 950, 1020, 1003)),
        "queue_warning": ("queue-warning", (782, 391, 1233, 467)),
        "normal_matching": ("matching", (11, 286, 177, 330)),
        "ready_case": ("ready-case", (849, 850, 1088, 930)),
        "ready_start": ("ready-start", (1503, 932, 1710, 1000)),
        "surrender": ("surrender-menu", (1342, 855, 1459, 931)),
        "result_first": ("result-first", (120, 954, 283, 1027)),
        "result_second": ("result-second", (2, 293, 157, 349)),
        "return_lobby": ("return-lobby", (880, 948, 1080, 1022)),
    },
    "hunter_rank": {
        "rank_entry": ("book-menu", (674, 270, 907, 339)),
        "traditional_rank": ("rank-mode", (441, 415, 700, 485)),
        "hunter_choice": ("role-select", (1377, 191, 1499, 244)),
        "start_case": ("start-case", (922, 950, 1020, 1003)),
        "rank_matching": ("matching", (11, 286, 177, 330)),
    },
}


for mode, templates in SPECS.items():
    folder = ROOT / mode
    for name, (screen, (left, top, right, bottom)) in templates.items():
        screenshot = cv2.imread(str(folder / "reference" / f"{screen}.png"))
        if screenshot is None or screenshot.shape[:2] != (1127, 1924):
            raise ValueError(f"Unexpected reference screenshot: {mode}/{screen}")
        frame = screenshot[45:1125, 2:1922]
        if not cv2.imwrite(str(folder / f"{name}.png"), frame[top:bottom, left:right]):
            raise OSError(f"Could not write {mode}/{name}.png")
