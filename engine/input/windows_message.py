from __future__ import annotations

import ctypes
import sys
from collections.abc import Callable
from ctypes import wintypes

from core.models import Point
from input.base import InputBackend
from input.windows_sendinput import _virtual_key

_WM_KEYDOWN = 0x0100
_WM_KEYUP = 0x0101
_WM_MOUSEMOVE = 0x0200
_WM_LBUTTONDOWN = 0x0201
_WM_LBUTTONUP = 0x0202
_WM_RBUTTONDOWN = 0x0204
_WM_RBUTTONUP = 0x0205
_MK_LBUTTON = 0x0001
_MK_RBUTTON = 0x0002


class WindowsMessageInput(InputBackend):
    """Send client-area input directly to the selected game window."""

    def __init__(self, hwnd_provider: Callable[[], int | None]) -> None:
        self._hwnd_provider = hwnd_provider
        self._point = Point(0, 0)
        self._user32 = None
        if sys.platform == "win32":
            self._user32 = ctypes.WinDLL("user32", use_last_error=True)
            self._user32.IsWindow.argtypes = [wintypes.HWND]
            self._user32.IsWindow.restype = wintypes.BOOL
            self._user32.PostMessageW.argtypes = [
                wintypes.HWND,
                wintypes.UINT,
                wintypes.WPARAM,
                wintypes.LPARAM,
            ]
            self._user32.PostMessageW.restype = wintypes.BOOL
            self._user32.MapVirtualKeyW.argtypes = [wintypes.UINT, wintypes.UINT]
            self._user32.MapVirtualKeyW.restype = wintypes.UINT

    def move_mouse(self, point: Point) -> None:
        self._point = point
        self._post(_WM_MOUSEMOVE, 0, _point_lparam(point))

    def mouse_down(self, button: str = "left") -> None:
        message, state = self._mouse_message(button, True)
        self._post(message, state, _point_lparam(self._point))

    def mouse_up(self, button: str = "left") -> None:
        message, _ = self._mouse_message(button, False)
        self._post(message, 0, _point_lparam(self._point))

    def key_down(self, key: str) -> None:
        self._key(key, False)

    def key_up(self, key: str) -> None:
        self._key(key, True)

    def _key(self, key: str, up: bool) -> None:
        if self._user32 is None:
            raise RuntimeError("window message input is only available on Windows")
        virtual_key = _virtual_key(key)
        scan_code = int(self._user32.MapVirtualKeyW(virtual_key, 0))
        lparam = 1 | (scan_code << 16)
        if up:
            lparam |= (1 << 30) | (1 << 31)
        self._post(_WM_KEYUP if up else _WM_KEYDOWN, virtual_key, lparam)

    def _post(self, message: int, wparam: int, lparam: int) -> None:
        if self._user32 is None:
            raise RuntimeError("window message input is only available on Windows")
        hwnd = self._hwnd_provider()
        if not hwnd or not self._user32.IsWindow(hwnd):
            raise RuntimeError("configured game window is not available")
        if not self._user32.PostMessageW(hwnd, message, wparam, lparam):
            raise ctypes.WinError(ctypes.get_last_error())

    @staticmethod
    def _mouse_message(button: str, down: bool) -> tuple[int, int]:
        messages = {
            ("left", True): (_WM_LBUTTONDOWN, _MK_LBUTTON),
            ("left", False): (_WM_LBUTTONUP, 0),
            ("right", True): (_WM_RBUTTONDOWN, _MK_RBUTTON),
            ("right", False): (_WM_RBUTTONUP, 0),
        }
        try:
            return messages[(button, down)]
        except KeyError as error:
            raise ValueError(f"unsupported mouse button: {button}") from error


def _point_lparam(point: Point) -> int:
    return (point.x & 0xFFFF) | ((point.y & 0xFFFF) << 16)
