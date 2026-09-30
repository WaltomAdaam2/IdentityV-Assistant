import random

from core.models import Point
from input.base import InputBackend
from input.controller import InputController
from input.windows_message import WindowsMessageInput, _curved_route, _point_lparam
from input.windows_sendinput import _virtual_key


class RecordingBackend(InputBackend):
    def __init__(self) -> None:
        self.calls: list[str] = []

    def move_mouse(self, point: Point) -> None: self.calls.append("move")
    def mouse_down(self, button: str = "left") -> None: self.calls.append("down")
    def mouse_up(self, button: str = "left") -> None: self.calls.append("up")
    def key_down(self, key: str) -> None: self.calls.append(f"key_down:{key}")
    def key_up(self, key: str) -> None: self.calls.append(f"key_up:{key}")


def test_release_all_releases_held_input() -> None:
    backend = RecordingBackend()
    controller = InputController(backend)
    controller.key_down("A")
    controller.mouse_down()
    controller.release_all()
    assert backend.calls == ["key_down:A", "down", "key_up:A", "up"]


def test_window_message_point_uses_client_coordinates() -> None:
    assert _point_lparam(Point(640, 360)) == (360 << 16) | 640


def test_window_message_supports_both_mouse_buttons() -> None:
    assert WindowsMessageInput._mouse_message("left", True)[0] == 0x0201
    assert WindowsMessageInput._mouse_message("right", True)[0] == 0x0204


def test_task_shortcuts_have_virtual_keys() -> None:
    assert [_virtual_key(key) for key in ("Q", "F", "ESC")] == [0x51, 0x46, 0x1B]


def test_mouse_route_is_curved_and_ends_at_target() -> None:
    route = _curved_route((0, 0), (1000, 500), random.Random(7))
    assert route[-1] == (1000, 500)
    assert any(y * 2 != x for x, y in route[:-1])
