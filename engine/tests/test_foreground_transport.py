import ctypes
from types import SimpleNamespace

import pytest

from core.models import Point
from input.controller import InputController
from input.windows_sendinput import WindowsClientInput, WindowsSendInput


@pytest.mark.parametrize("key,scan", [
    ("w", 0x11), ("a", 0x1E), ("s", 0x1F), ("d", 0x20),
    ("f", 0x21), ("r", 0x13), ("t", 0x14), ("g", 0x22),
    ("q", 0x10), ("e", 0x12), ("v", 0x2F), ("space", 0x39),
    ("esc", 0x01), ("alt", 0x38), ("1", 0x02),
])
def test_every_game_key_has_matching_physical_down_up(key, scan):
    down, up = [WindowsSendInput._keyboard_event(key, up) for up in (False, True)]
    assert down.data.ki.wVk == up.data.ki.wVk == 0
    assert down.data.ki.wScan == up.data.ki.wScan == scan
    assert down.data.ki.dwFlags == 8 and up.data.ki.dwFlags == 10


def test_failed_sendinput_is_not_silent_and_does_not_report_stale_last_error():
    backend = object.__new__(WindowsSendInput)
    backend._expected_hwnd = 42
    backend._user32 = SimpleNamespace(GetForegroundWindow=lambda: 42, SendInput=lambda *_: 0)
    ctypes.set_last_error(999)
    with pytest.raises(OSError, match="SendInput 0/1") as caught:
        backend.key_down("w")
    assert caught.value.errno == 0


@pytest.mark.parametrize("kind", ["key", "mouse"])
def test_tap_release_failure_stays_owned_by_original_backend_for_retry(monkeypatch, kind):
    calls, delays = [], []
    fail = [True]

    def up(value):
        calls.append(("up", value))
        if fail[0]:
            raise OSError("release blocked")

    backend = SimpleNamespace(
        key_down=lambda key: calls.append(("down", key)), key_up=up,
        mouse_down=lambda button: calls.append(("down", button)), mouse_up=up,
        move_mouse=lambda point: None,
    )
    controller = InputController(backend)
    monkeypatch.setattr("input.controller.sleep", delays.append)
    with pytest.raises(OSError, match="release blocked"):
        controller.press("f") if kind == "key" else controller.click(Point(50, 50))
    assert delays and delays[0] >= 0.08
    assert controller.pressed_keys or controller.mouse_buttons_down
    controller.backend = SimpleNamespace()  # A completed task restored the login backend.
    fail[0] = False
    controller.release_all()
    assert not controller.pressed_keys and not controller.mouse_buttons_down
    assert len([call for call in calls if call[0] == "up"]) == 2


def test_failed_down_still_gets_release_and_failed_release_remains_tracked(monkeypatch):
    def fail(*_):
        raise OSError("blocked")

    controller = InputController(SimpleNamespace(key_down=fail, key_up=fail))
    with pytest.raises(OSError):
        controller.press("w")
    assert controller.pressed_keys == {"w"}


def test_release_modifier_after_combination_key():
    released = []
    controller = InputController(SimpleNamespace(key_down=lambda key: None, key_up=released.append))
    controller.key_down("alt")
    controller.key_down("1")
    controller.release_all()
    assert released == ["1", "alt"]


def test_client_input_converts_coordinates_smoothly_and_releases_after_window_disappears(monkeypatch):
    hwnd = [42]
    moves, released = [], []
    backend = SimpleNamespace(
        _expected_hwnd=42, client_to_screen=lambda p: Point(p.x - 1600, p.y + 100),
        cursor_position=lambda: Point(-1500, 200), move_mouse=moves.append,
        key_up=released.append,
    )
    adapter = WindowsClientInput(lambda: hwnd[0])
    adapter._backend = backend
    monkeypatch.setattr("input.windows_sendinput.sleep", lambda _: None)
    adapter.move_mouse(Point(800, 450))
    assert moves[-1] == Point(-800, 550)
    assert len(moves) > 10
    hwnd[0] = None
    adapter.key_up("w")
    assert released == ["w"]


def test_absolute_input_clamps_to_virtual_desktop_and_rejects_missing_dimensions():
    backend = object.__new__(WindowsSendInput)
    metrics = {76: -1920, 77: 0, 78: 3840, 79: 1080}
    backend._user32 = SimpleNamespace(GetSystemMetrics=metrics.__getitem__)
    moves = []
    backend._mouse = lambda flags, x, y: moves.append((x, y))
    backend.move_mouse(Point(-9000, 9000))
    assert moves == [(0, 65535)]
    metrics[78] = 0
    with pytest.raises(RuntimeError):
        backend.move_mouse(Point(10, 10))
