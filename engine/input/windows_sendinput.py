from __future__ import annotations

import ctypes
import sys
from ctypes import wintypes

from core.models import Point
from input.base import InputBackend


_MOUSE_MOVE = 0x0001
_MOUSE_LEFT_DOWN = 0x0002
_MOUSE_LEFT_UP = 0x0004
_MOUSE_RIGHT_DOWN = 0x0008
_MOUSE_RIGHT_UP = 0x0010
_MOUSE_ABSOLUTE = 0x8000
_MOUSE_VIRTUAL_DESKTOP = 0x4000
_KEY_UP = 0x0002


class _MouseInput(ctypes.Structure):
    _fields_ = [
        ("dx", wintypes.LONG), ("dy", wintypes.LONG),
        ("mouseData", wintypes.DWORD), ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD), ("dwExtraInfo", ctypes.c_size_t),
    ]


class _KeyboardInput(ctypes.Structure):
    _fields_ = [
        ("wVk", wintypes.WORD), ("wScan", wintypes.WORD),
        ("dwFlags", wintypes.DWORD), ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.c_size_t),
    ]


class _InputUnion(ctypes.Union):
    _fields_ = [("mi", _MouseInput), ("ki", _KeyboardInput)]


class _Input(ctypes.Structure):
    _fields_ = [("type", wintypes.DWORD), ("data", _InputUnion)]


class WindowsSendInput(InputBackend):
    """Send normal foreground mouse and keyboard input through Win32 SendInput."""

    def __init__(self, expected_hwnd: int | None = None) -> None:
        if sys.platform != "win32":
            raise RuntimeError("SendInput is only available on Windows")
        self._user32 = ctypes.WinDLL("user32", use_last_error=True)
        self._expected_hwnd = expected_hwnd
        self._user32.SendInput.argtypes = [wintypes.UINT, ctypes.POINTER(_Input), ctypes.c_int]
        self._user32.SendInput.restype = wintypes.UINT
        self._user32.GetForegroundWindow.argtypes = []
        self._user32.GetForegroundWindow.restype = wintypes.HWND
        self._user32.GetCursorPos.argtypes = [ctypes.POINTER(wintypes.POINT)]
        self._user32.GetCursorPos.restype = wintypes.BOOL
        self._user32.GetSystemMetrics.argtypes = [ctypes.c_int]
        self._user32.GetSystemMetrics.restype = ctypes.c_int

    def _send(self, item: _Input, require_foreground: bool = True) -> None:
        if require_foreground and self._expected_hwnd is not None:
            if self._user32.GetForegroundWindow() != self._expected_hwnd:
                raise RuntimeError("Game window lost foreground before input")
        if self._user32.SendInput(1, ctypes.byref(item), ctypes.sizeof(_Input)) != 1:
            raise OSError(ctypes.get_last_error(), "SendInput failed")

    def _mouse(self, flags: int, x: int = 0, y: int = 0, require_foreground: bool = True) -> None:
        self._send(_Input(0, _InputUnion(mi=_MouseInput(x, y, 0, flags, 0, 0))), require_foreground)

    def move_mouse(self, point: Point) -> None:
        left = self._user32.GetSystemMetrics(76)
        top = self._user32.GetSystemMetrics(77)
        width = self._user32.GetSystemMetrics(78)
        height = self._user32.GetSystemMetrics(79)
        x = round((point.x - left) * 65535 / max(width - 1, 1))
        y = round((point.y - top) * 65535 / max(height - 1, 1))
        self._mouse(_MOUSE_MOVE | _MOUSE_ABSOLUTE | _MOUSE_VIRTUAL_DESKTOP, x, y)

    def move_relative(self, dx: int, dy: int) -> None:
        self._mouse(_MOUSE_MOVE, dx, dy)

    def cursor_position(self) -> Point:
        position = wintypes.POINT()
        if not self._user32.GetCursorPos(ctypes.byref(position)):
            raise OSError(ctypes.get_last_error(), "GetCursorPos failed")
        return Point(position.x, position.y)

    def mouse_down(self, button: str = "left") -> None:
        flags = {"left": _MOUSE_LEFT_DOWN, "right": _MOUSE_RIGHT_DOWN}
        if button not in flags:
            raise ValueError(f"unsupported mouse button: {button}")
        self._mouse(flags[button])

    def mouse_up(self, button: str = "left") -> None:
        flags = {"left": _MOUSE_LEFT_UP, "right": _MOUSE_RIGHT_UP}
        if button not in flags:
            raise ValueError(f"unsupported mouse button: {button}")
        self._mouse(flags[button], require_foreground=False)

    def key_down(self, key: str) -> None:
        self._send(_Input(1, _InputUnion(ki=_KeyboardInput(_virtual_key(key), 0, 0, 0, 0))))

    def key_up(self, key: str) -> None:
        self._send(_Input(1, _InputUnion(ki=_KeyboardInput(_virtual_key(key), 0, _KEY_UP, 0, 0))), False)


def _virtual_key(key: str) -> int:
    normalized = key.upper()
    named = {"ENTER": 0x0D, "ESC": 0x1B, "SPACE": 0x20, "TAB": 0x09, "ALT": 0x12}
    if normalized in named:
        return named[normalized]
    if len(normalized) == 1 and normalized.isalnum():
        return ord(normalized)
    raise ValueError(f"unsupported key: {key}")
