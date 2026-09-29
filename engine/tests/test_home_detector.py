import numpy as np

from core.models import Point
from vision.home_detector import detect_home_book, home_book_click_point, home_book_samples


def _matching_frame(width: int = 1920, height: int = 1080):
    image = np.zeros((height, width, 3), dtype=np.uint8)
    for point, color in home_book_samples():
        x = round(point.x * width / 1920)
        y = round(point.y * height / 1080)
        image[y, x] = color
    return image


def test_home_detector_accepts_eight_of_ten_pixels() -> None:
    image = _matching_frame()
    for point, _ in home_book_samples()[:2]:
        image[point.y, point.x] = 0
    result = detect_home_book(image)
    assert result.found is True
    assert result.matches == 8


def test_home_detector_rejects_seven_of_ten_pixels() -> None:
    image = _matching_frame()
    for point, _ in home_book_samples()[:3]:
        image[point.y, point.x] = 0
    result = detect_home_book(image)
    assert result.found is False
    assert result.matches == 7


def test_home_detector_scales_samples_and_click_point() -> None:
    image = _matching_frame(1600, 900)
    assert detect_home_book(image).found is True
    assert home_book_click_point(1600, 900) == Point(1500, 633)
