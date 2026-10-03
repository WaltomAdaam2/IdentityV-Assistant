import random
import sys
from types import SimpleNamespace

import pytest

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


def test_failed_key_release_still_releases_other_input_and_can_retry() -> None:
    class FailingBackend(RecordingBackend):
        fail = True

        def key_up(self, key):
            if key == "A" and self.fail:
                raise RuntimeError("release failed")
            super().key_up(key)

    backend = FailingBackend()
    controller = InputController(backend)
    controller.key_down("A")
    controller.key_down("D")
    controller.mouse_down()
    with pytest.raises(RuntimeError, match="release failed"):
        controller.release_all()
    assert "key_up:D" in backend.calls and "up" in backend.calls
    assert controller.pressed_keys == {"A"}
    assert not controller.mouse_buttons_down
    backend.fail = False
    controller.release_all()
    assert not controller.pressed_keys


def test_window_message_point_uses_client_coordinates() -> None:
    assert _point_lparam(Point(640, 360)) == (360 << 16) | 640


def test_window_message_supports_both_mouse_buttons() -> None:
    assert WindowsMessageInput._mouse_message("left", True)[0] == 0x0201
    assert WindowsMessageInput._mouse_message("right", True)[0] == 0x0204


def test_task_shortcuts_have_virtual_keys() -> None:
    assert [_virtual_key(key) for key in ("Q", "F", "ESC", "ALT", "1")] == [0x51, 0x46, 0x1B, 0x12, 0x31]


def test_mouse_route_is_curved_and_ends_at_target() -> None:
    route = _curved_route((0, 0), (1000, 500), random.Random(7))
    assert route[-1] == (1000, 500)
    assert any(y * 2 != x for x, y in route[:-1])


def test_login_click_does_not_snap_cursor_back(monkeypatch) -> None:
    position = [(50, 60)]
    moves = []
    messages = []

    def move(point):
        position[0] = point
        moves.append(point)

    api = SimpleNamespace(
        GetCursorPos=lambda: position[0], SetCursorPos=move,
        PostMessage=lambda *args: messages.append(args),
    )
    constants = SimpleNamespace(WM_ACTIVATE=6, WA_ACTIVE=1)
    gui = SimpleNamespace(ClientToScreen=lambda hwnd, point: (point[0] + 10, point[1] + 20), IsIconic=lambda hwnd: False)
    backend = WindowsMessageInput(lambda: 123)
    monkeypatch.setitem(sys.modules, "win32api", api)
    monkeypatch.setattr(backend, "_target", lambda: (api, constants, gui, 123))
    monkeypatch.setattr("input.windows_message.sleep", lambda _: None)
    backend.move_mouse(Point(900, 500))
    backend.mouse_down()
    backend.mouse_up()
    assert position[0] == (910, 520)
    assert moves[-1] == (910, 520)
    assert all(point != (50, 60) for point in moves)
    assert messages[-2][1] == 0x0201 and messages[-1][1] == 0x0202
    assert len(moves) > 30
    assert max(abs(b[0] - a[0]) for a, b in zip(moves, moves[1:])) < 50
