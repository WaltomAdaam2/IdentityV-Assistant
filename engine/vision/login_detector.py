from __future__ import annotations

from typing import Any

_BASE_WIDTH = 1920
_BASE_HEIGHT = 1080


def _region(image: Any, box: tuple[int, int, int, int]) -> Any:
    height, width = image.shape[:2]
    left, top, right, bottom = box
    x1 = max(0, round(left * width / _BASE_WIDTH))
    y1 = max(0, round(top * height / _BASE_HEIGHT))
    x2 = min(width, round(right * width / _BASE_WIDTH))
    y2 = min(height, round(bottom * height / _BASE_HEIGHT))
    return image[y1:y2, x1:x2]


def _metrics(image: Any, box: tuple[int, int, int, int]) -> tuple[float, float, float]:
    import cv2
    import numpy as np

    area = _region(image, box)
    if area.size == 0:
        return 0.0, 0.0, 0.0
    gray = cv2.cvtColor(area, cv2.COLOR_BGR2GRAY)
    hsv = cv2.cvtColor(area, cv2.COLOR_BGR2HSV)
    bright = float(np.mean(gray > 150))
    edges = float(np.mean(cv2.Canny(gray, 60, 140) > 0))
    orange = float(
        np.mean(
            (hsv[:, :, 0] >= 5)
            & (hsv[:, :, 0] <= 30)
            & (hsv[:, :, 1] >= 100)
            & (hsv[:, :, 2] >= 150)
        )
    )
    return bright, edges, orange


def is_platform_choice(image: Any) -> bool:
    android_bright, _, _ = _metrics(image, (420, 480, 790, 680))
    ios_bright, _, _ = _metrics(image, (1120, 480, 1500, 680))
    title_bright, _, _ = _metrics(image, (450, 120, 1470, 330))
    _, divider_edges, _ = _metrics(image, (955, 400, 965, 930))
    return (
        0.02 <= android_bright <= 0.15
        and 0.01 <= ios_bright <= 0.15
        and 0.03 <= title_bright <= 0.20
        and divider_edges >= 0.14
    )


def is_announcement(image: Any) -> bool:
    center_bright, _, _ = _metrics(image, (700, 350, 1220, 730))
    _, exit_edges, _ = _metrics(image, (20, 15, 130, 150))
    return center_bright >= 0.62 and exit_edges >= 0.025


def is_login_landing(image: Any) -> bool:
    badge_bright, _, badge_orange = _metrics(image, (1750, 870, 1910, 1070))
    return badge_bright >= 0.30 and badge_orange >= 0.15


def is_activity_popup(image: Any) -> bool:
    center_bright, center_edges, _ = _metrics(image, (700, 350, 1220, 730))
    _, exit_edges, _ = _metrics(image, (20, 15, 130, 150))
    _, _, badge_orange = _metrics(image, (1750, 870, 1910, 1070))
    return (
        0.08 <= center_bright <= 0.55
        and center_edges >= 0.12
        and exit_edges >= 0.03
        and badge_orange < 0.08
    )
