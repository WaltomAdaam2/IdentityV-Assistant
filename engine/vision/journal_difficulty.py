"""Read the visible floor selector, independently of map geometry or user filters."""
from functools import lru_cache

import cv2
import numpy as np

from core.resources import resource_root


@lru_cache(maxsize=1)
def _references():
    root = resource_root() / "assets/templates/journal"
    images = [cv2.imdecode(np.frombuffer((root / name).read_bytes(), np.uint8), 0)
              for name in ("hard_floors.png", "nightmare_floors.png")]
    if any(image is None for image in images):
        raise ValueError("地图楼层识别资源损坏 / Floor selector reference is damaged")
    return images[0], images[1][:, :55]


def detect_difficulty(frame: np.ndarray) -> tuple[str, dict]:
    # Normalize only this small HUD analysis. Matching retains the original pixels.
    gray = cv2.cvtColor(frame[:, :, :3], cv2.COLOR_BGR2GRAY)
    gray = cv2.resize(gray, (1600, 900), interpolation=cv2.INTER_AREA)
    pair, basement = _references()
    roi = gray[20:115, 1080:1450]
    _, score, _, point = cv2.minMaxLoc(cv2.matchTemplate(roi, pair, cv2.TM_CCOEFF_NORMED))
    x, y = point[0] + 1080, point[1] + 20
    found = gray[y:y + pair.shape[0], x:x + pair.shape[1]]
    # A textured selected tile alone must not stand in for two visible numbers.
    digits = [float(cv2.matchTemplate(found[5:35, left:right], pair[5:35, left:right],
                                     cv2.TM_CCOEFF_NORMED)[0, 0])
              for left, right in ((18, 38), (76, 98))]
    left = gray[y:y + 41, x - 62:x - 1]
    minus_score = float(cv2.minMaxLoc(cv2.matchTemplate(left, basement, cv2.TM_CCOEFF_NORMED))[1])
    background = gray[y:y + 41, x - 100:x - 65]
    empty = float(left.std()) < 4 and abs(float(left.mean()) - float(background.mean())) < 5
    details = {"pair_score": float(score), "digit_scores": digits, "basement_score": minus_score,
               "left_background_clear": empty, "normalized_selector": [x, y, pair.shape[1], pair.shape[0]]}
    if score >= .90 and min(digits) >= .80:
        if minus_score >= .80:
            return "nightmare", details
        if empty:
            return "hard", details
    raise ValueError("无法确认地图楼层标志，请打开完整的一楼地图后重试 / Open the full first-floor map so its floor labels are visible, then retry")
