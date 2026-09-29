from core.models import Point
from input.base import InputBackend
from input.controller import InputController
from input.windows_message import _point_lparam


class RecordingBackend(InputBackend):
    def __init__(self) -> None:
        self.calls: list[str] = []

    def move_mouse(self, point: Point) -> None: self.calls.append("move")
    def mouse_down(self, button: str = "left") -> None: self.calls.append("down")
    def mouse_up(self, button: str = "left") -> None: self.calls.append("up")
    def key_down(self, key: str) -> None: self.calls.append(f"key_down:{key}")
    def key_up(self, key: str) -> None: self.calls.append(f"key_up:{key}")


def test_dry_run_records_without_touching_backend() -> None:
    backend = RecordingBackend()
    recorded: list[dict] = []
    controller = InputController(backend, dry_run=True, on_action=recorded.append)
    controller.click(Point(1, 2))
    controller.key_down("A")
    controller.release_all()
    assert backend.calls == []
    assert controller.pressed_keys == set()
    assert [item["action"] for item in recorded] == ["click", "key_down", "release_all"]


def test_release_all_releases_held_input() -> None:
    backend = RecordingBackend()
    controller = InputController(backend, dry_run=False)
    controller.key_down("A")
    controller.mouse_down()
    controller.release_all()
    assert backend.calls == ["key_down:A", "down", "key_up:A", "up"]


def test_window_message_point_uses_client_coordinates() -> None:
    assert _point_lparam(Point(640, 360)) == (360 << 16) | 640
