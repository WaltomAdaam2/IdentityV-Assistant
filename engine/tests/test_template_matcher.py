import pytest
from pathlib import Path

pytest.importorskip("numpy")
pytest.importorskip("cv2")

from vision.template_matcher import TemplateMatcher


def test_template_matcher_finds_fixture_pattern() -> None:
    import cv2

    fixtures = Path(__file__).parent / "fixtures"
    image = cv2.imread(str(fixtures / "mock_frame.pgm"), cv2.IMREAD_GRAYSCALE)
    template = cv2.imread(str(fixtures / "mock_template.pgm"), cv2.IMREAD_GRAYSCALE)
    result = TemplateMatcher().match(image, template, threshold=0.99)
    assert result.found
    assert result.bbox == (4, 3, 3, 3)
