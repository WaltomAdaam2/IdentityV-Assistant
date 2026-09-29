from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from core.models import Point

_REFERENCE_WIDTH = 1920
_REFERENCE_HEIGHT = 1080
_BOOK_CLICK = Point(1800, 760)
_BOOK_PIXELS: tuple[tuple[Point, tuple[int, int, int]], ...] = (
    (Point(1826, 724), (203, 179, 163)),
    (Point(1806, 734), (200, 175, 150)),
    (Point(1776, 739), (165, 145, 126)),
    (Point(1796, 744), (193, 170, 152)),
    (Point(1786, 754), (179, 154, 128)),
    (Point(1826, 754), (166, 147, 127)),
    (Point(1766, 764), (164, 139, 116)),
    (Point(1841, 764), (163, 138, 114)),
    (Point(1731, 774), (168, 147, 138)),
    (Point(1866, 784), (192, 170, 151)),
)


@dataclass(frozen=True)
class HomeDetection:
    found: bool
    matches: int
    required: int = 8
    total: int = len(_BOOK_PIXELS)


def detect_home_book(image: Any, tolerance: int = 35) -> HomeDetection:
    if image is None or len(image.shape) < 2:
        raise ValueError("a captured image is required")
    height, width = image.shape[:2]
    matches = 0
    for point, expected_bgr in _BOOK_PIXELS:
        x = min(width - 1, round(point.x * width / _REFERENCE_WIDTH))
        y = min(height - 1, round(point.y * height / _REFERENCE_HEIGHT))
        actual = image[y, x][:3]
        if all(abs(int(actual[index]) - expected_bgr[index]) <= tolerance for index in range(3)):
            matches += 1
    return HomeDetection(matches >= 8, matches)


def home_book_click_point(width: int, height: int) -> Point:
    return Point(
        round(_BOOK_CLICK.x * width / _REFERENCE_WIDTH),
        round(_BOOK_CLICK.y * height / _REFERENCE_HEIGHT),
    )


def home_book_samples() -> tuple[tuple[Point, tuple[int, int, int]], ...]:
    return _BOOK_PIXELS
