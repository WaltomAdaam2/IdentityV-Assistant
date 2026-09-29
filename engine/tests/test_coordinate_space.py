from core.models import CoordinateSpace, Point, Rect


def test_coordinate_scales_from_canonical_client_space() -> None:
    space = CoordinateSpace()
    rect = Rect(100, 200, 960, 540)
    assert space.to_client(Point(960, 540), rect) == Point(480, 270)
    assert space.to_screen(Point(960, 540), rect) == Point(580, 470)
