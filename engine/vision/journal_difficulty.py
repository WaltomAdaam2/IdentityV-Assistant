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


def _horizontal(gray: np.ndarray) -> tuple[str, dict]:
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


def _label(tile: np.ndarray) -> tuple[bool, bool] | None:
    """Read a centered numeral silhouette and an optional separate minus stroke."""
    height, width = tile.shape
    if height < 40 or width < 25:
        return None
    center = tile[round(height * .15):round(height * .85), round(width * .15):round(width * .85)]
    median = float(np.median(tile))
    mask = center < median - 45 if median > 60 else center > max(55, median + 20)
    _, _, stats, _ = cv2.connectedComponentsWithStats(mask.astype(np.uint8))
    digits = [s for s in stats[1:] if height * .15 <= s[3] <= height * .45
              and 2 <= s[2] <= width * .4 and s[4] >= 12]
    if len(digits) != 1:
        return None
    digit = digits[0]
    # Reject icons and dense scenery, not just empty neighboring slots.
    if mask.mean() > .22:
        return None
    minus = any(s[2] >= width * .13 and 1 <= s[3] <= height * .08 and s[4] >= 8
                and s[0] + s[2] <= digit[0] + 1
                and digit[1] < s[1] < digit[1] + digit[3] for s in stats[1:])
    return minus, bool(digit[2] < digit[3] * .4)


def _vertical(gray: np.ndarray, season: str) -> tuple[str | None, str, dict] | None:
    # Find compact highlighted tiles throughout the image; no stored desktop ROI.
    mask = (gray > 55).astype(np.uint8) * 255
    mask = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
    mask = cv2.morphologyEx(mask, cv2.MORPH_OPEN, np.ones((35, 23), np.uint8))
    _, _, stats, _ = cv2.connectedComponentsWithStats(mask)
    found = []
    for x, y, width, height, area in stats[1:]:
        x, y, width, height = map(int, (x, y, width, height))
        if not (30 <= width <= 85 and 45 <= height <= 120
                and 1.15 <= height / width <= 1.65 and area / (width * height) > .8):
            continue
        tile = gray[y:y + height, x:x + width]
        highlight = float(np.median(tile))
        if highlight < 65 or float((tile < 55).mean()) >= .4 or _label(tile) is None:
            continue
        # Slot height includes the small gap. Check neighbors on both sides so a
        # partial/occluded group cannot silently become a different floor index.
        slots = {}
        for offset in range(-4, 5):
            top = y + offset * height
            if top < 0 or top + height > gray.shape[0]:
                continue
            cell = gray[top:top + height, x:x + width]
            label = _label(cell)
            if label is not None:
                slots[offset] = (label, float(np.median(cell)))
        start = end = 0
        while start - 1 in slots:
            start -= 1
        while end + 1 in slots:
            end += 1
        count = end - start + 1
        if count not in ((3, 4) if season == "s1" else (2, 3)):
            continue
        labels = [slots[i][0] for i in range(start, end + 1)]
        medians = [slots[i][1] for i in range(start, end + 1)]
        selected = -start
        if any(value * 1.5 >= highlight for i, value in enumerate(medians) if i != selected):
            continue
        # Both outer slots must be plain surrounding background. In particular,
        # an obscured -1 must not be interpreted as a two-button hard selector.
        clear = True
        for offset in (start - 1, end + 1):
            top = y + offset * height
            if top < 0 or top + height > gray.shape[0] or x + width + 22 > gray.shape[1]:
                clear = False
                break
            outside = gray[top + 8:top + height - 8, x + 8:x + width - 8]
            background = gray[top + 8:top + height - 8, x + width + 8:x + width + 22]
            if outside.std() > 5 or abs(float(np.median(outside)) - float(np.median(background))) > 5:
                clear = False
        if not clear:
            continue
        if season == "s1":
            if not all(label[0] for label in labels) or not labels[0][1] or any(label[1] for label in labels[1:]):
                continue
            difficulty, floor = None, str(selected + 1)
        else:
            expected = [(False, False), (False, True)] + ([(True, True)] if count == 3 else [])
            if labels != expected:
                continue
            difficulty = "nightmare" if count == 3 else "hard"
            floor = ("2", "1", "b1f")[selected]
        found.append((difficulty, floor, {"layout": "vertical", "slot_count": count,
            "selected_slot": selected, "floor": floor, "slot_medians": medians,
            "normalized_selector": [x, y + start * height, width, count * height]}))
    if len(found) > 1:
        raise ValueError("楼层按钮区域不唯一，请打开完整地图后重试 / Multiple floor selectors; open the full map and retry")
    return found[0] if found else None


def _gray(frame: np.ndarray) -> np.ndarray:
    # Only HUD analysis is normalized. Geometric matching retains original pixels.
    return cv2.resize(cv2.cvtColor(frame[:, :, :3], cv2.COLOR_BGR2GRAY), (1600, 900), interpolation=cv2.INTER_AREA)


def detect_difficulty(frame: np.ndarray) -> tuple[str, dict]:
    gray = _gray(frame)
    result = _vertical(gray, "s0")
    if result is None:
        return _horizontal(gray)
    difficulty, floor, details = result
    if floor != "1":
        raise ValueError("S0 仅支持一楼识图，请切换游戏地图至一楼 / S0 recognition requires the first-floor map")
    return difficulty, details


def detect_s1_floor(frame: np.ndarray) -> tuple[str, dict]:
    result = _vertical(_gray(frame), "s1")
    if result is None:
        raise ValueError("无法确认 S1 楼层，请打开完整地图并显示楼层按钮后重试 / Cannot confirm S1 floor; show the full map and floor selector")
    _, floor, details = result
    if floor == "4":
        raise ValueError("地下4层暂不支持，请切换至地下1、2或3层 / B4 is not supported; select B1, B2 or B3")
    return floor, details
