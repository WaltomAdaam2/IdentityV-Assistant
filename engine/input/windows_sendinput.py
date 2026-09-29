from __future__ import annotations

import ctypes
import sys

from core.models import Point
from input.base import InputBackend

_MOUSEEVENTF_MOVE = 0x0001
_MOUSEEVENTF_ABSOLUTE = 0x8000
_MOUSEEVENTF_VIRTUALDESK = 0x4000
_MOUSEEVENTF_LEFTDOWN = 0x0002
_MOUSEEVENTF_LEFTUP = 0x0004
_MOUSEEVENTF_RIGHTDOWN = 0x0008
_MOUSEEVENTF_RIGHTUP = 0x0010
_KEYEVENTF_KEYUP = 0x0002
_INPUT_MOUSE = 0
_INPUT_KEYBOARD = 1


class WindowsSendInput(InputBackend):
    """Visible, normal Win32 input; intentionally no process or memory access."""

    def _available(self) -> None:
        if sys.platform != "win32":
            raise RuntimeError("SendInput is only available on Windows")

    def move_mouse(self, point: Point) -> None:
        self._available()
        left = ctypes.windll.user32.GetSystemMetrics(76)
        top = ctypes.windll.user32.GetSystemMetrics(77)
        width = ctypes.windll.user32.GetSystemMetrics(78)
        height = ctypes.windll.user32.GetSystemMetrics(79)
        ctypes.windll.user32.mouse_event(
            _MOUSEEVENTF_MOVE | _MOUSEEVENTF_ABSOLUTE | _MOUSEEVENTF_VIRTUALDESK,
            round((point.x - left) * 65535 / max(width - 1, 1)),
            round((point.y - top) * 65535 / max(height - 1, 1)),
            0,
            0,
        )

    def mouse_down(self, button: str = "left") -> None:
        self._mouse(button, True)

    def mouse_up(self, button: str = "left") -> None:
        self._mouse(button, False)

    def _mouse(self, button: str, down: bool) -> None:
        self._available()
        flags = {("left", True): _MOUSEEVENTF_LEFTDOWN, ("left", False): _MOUSEEVENTF_LEFTUP, ("right", True): _MOUSEEVENTF_RIGHTDOWN, ("right", False): _MOUSEEVENTF_RIGHTUP}
        if (button, down) not in flags:
            raise ValueError(f"unsupported mouse button: {button}")
        ctypes.windll.user32.mouse_event(flags[(button, down)], 0, 0, 0, 0)

    def key_down(self, key: str) -> None:
        self._key(key, False)

    def key_up(self, key: str) -> None:
        self._key(key, True)

    def _key(self, key: str, up: bool) -> None:
        self._available()
        virtual_key = _virtual_key(key)
        ctypes.windll.user32.keybd_event(virtual_key, 0, _KEYEVENTF_KEYUP if up else 0, 0)


def _virtual_key(key: str) -> int:
    normalized = key.upper()
    named = {"ENTER": 0x0D, "ESC": 0x1B, "SPACE": 0x20, "TAB": 0x09}
    if normalized in named:
        return named[normalized]
    if len(normalized) == 1 and normalized.isalnum():
        return ord(normalized)
    raise ValueError(f"unsupported key: {key}")
