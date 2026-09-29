from __future__ import annotations

from collections.abc import Callable
from time import sleep

from core.models import Point
from input.base import InputBackend
from input.windows_sendinput import _virtual_key

_WM_KEYDOWN = 0x0100
_WM_KEYUP = 0x0101
_WM_LBUTTONDOWN = 0x0201
_WM_LBUTTONUP = 0x0202
_WM_RBUTTONDOWN = 0x0204
_WM_RBUTTONUP = 0x0205
_MK_LBUTTON = 0x0001
_MK_RBUTTON = 0x0002


class WindowsMessageInput(InputBackend):
    """Deliver background input through the target window's Win32 message queue."""

    def __init__(self, hwnd_provider: Callable[[], int | None]) -> None:
        self._hwnd_provider = hwnd_provider
        self._point = Point(0, 0)
        self._saved_cursor: tuple[int, int] | None = None

    def move_mouse(self, point: Point) -> None:
        win32api, win32con, win32gui, hwnd = self._target()
        self._point = point
        if self._saved_cursor is None:
            self._saved_cursor = win32api.GetCursorPos()
        screen_point = win32gui.ClientToScreen(hwnd, (point.x, point.y))
        win32api.SetCursorPos(screen_point)
        if win32gui.IsIconic(hwnd):
            win32gui.ShowWindow(hwnd, win32con.SW_SHOWNOACTIVATE)
        win32api.PostMessage(hwnd, win32con.WM_ACTIVATE, win32con.WA_ACTIVE, 0)

    def mouse_down(self, button: str = "left") -> None:
        import win32api

        try:
            _, _, _, hwnd = self._target()
            message, state = self._mouse_message(button, True)
            win32api.PostMessage(hwnd, message, state, _point_lparam(self._point))
            sleep(0.03)
        except Exception:
            self._restore_cursor(win32api)
            raise

    def mouse_up(self, button: str = "left") -> None:
        import win32api

        try:
            _, _, _, hwnd = self._target()
            message, _ = self._mouse_message(button, False)
            win32api.PostMessage(hwnd, message, 0, _point_lparam(self._point))
            sleep(0.03)
        finally:
            self._restore_cursor(win32api)

    def key_down(self, key: str) -> None:
        self._key(key, False)

    def key_up(self, key: str) -> None:
        self._key(key, True)

    def _key(self, key: str, up: bool) -> None:
        win32api, win32con, win32gui, hwnd = self._target()
        if win32gui.IsIconic(hwnd):
            win32gui.ShowWindow(hwnd, win32con.SW_SHOWNOACTIVATE)
        win32api.PostMessage(hwnd, win32con.WM_ACTIVATE, win32con.WA_ACTIVE, 0)
        virtual_key = _virtual_key(key)
        scan_code = int(win32api.MapVirtualKey(virtual_key, 0)) & 0xFF
        lparam = 1 | (scan_code << 16)
        if up:
            lparam |= (1 << 30) | (1 << 31)
        win32api.PostMessage(hwnd, _WM_KEYUP if up else _WM_KEYDOWN, virtual_key, lparam)

    def _target(self):
        import win32api
        import win32con
        import win32gui

        hwnd = self._hwnd_provider()
        if not hwnd or not win32gui.IsWindow(hwnd):
            raise RuntimeError("configured game window is not available")
        return win32api, win32con, win32gui, hwnd

    def _restore_cursor(self, win32api) -> None:
        if self._saved_cursor is not None:
            win32api.SetCursorPos(self._saved_cursor)
            self._saved_cursor = None

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
