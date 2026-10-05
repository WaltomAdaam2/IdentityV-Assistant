from __future__ import annotations

import ctypes
import sys
from collections.abc import Callable
from ctypes import wintypes
from time import sleep

from core.models import Point
from input.base import InputBackend
from input.mouse_path import curved_route, route_duration


_MOUSE_MOVE = 0x0001
_MOUSE_LEFT_DOWN = 0x0002
_MOUSE_LEFT_UP = 0x0004
_MOUSE_RIGHT_DOWN = 0x0008
_MOUSE_RIGHT_UP = 0x0010
_MOUSE_ABSOLUTE = 0x8000
_MOUSE_VIRTUAL_DESKTOP = 0x4000
_KEY_UP = 0x0002
_KEY_SCANCODE = 0x0008


class ForegroundLostError(RuntimeError):
    """Foreground changed between task validation and an input event."""


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
        self._user32.ClientToScreen.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.POINT)]
        self._user32.ClientToScreen.restype = wintypes.BOOL

    def _send(self, item: _Input, require_foreground: bool = True) -> None:
        if require_foreground and self._expected_hwnd is not None:
            if self._user32.GetForegroundWindow() != self._expected_hwnd:
                raise ForegroundLostError("Game window lost foreground before input")
        ctypes.set_last_error(0)
        inserted = self._user32.SendInput(1, ctypes.byref(item), ctypes.sizeof(_Input))
        if inserted != 1:
            raise OSError(ctypes.get_last_error(), f"前台输入发送失败（SendInput {inserted}/1）；请确认游戏与助手权限一致，且游戏未处于锁屏桌面。")

    def _mouse(self, flags: int, x: int = 0, y: int = 0, require_foreground: bool = True) -> None:
        self._send(_Input(0, _InputUnion(mi=_MouseInput(x, y, 0, flags, 0, 0))), require_foreground)

    def move_mouse(self, point: Point) -> None:
        left = self._user32.GetSystemMetrics(76)
        top = self._user32.GetSystemMetrics(77)
        width = self._user32.GetSystemMetrics(78)
        height = self._user32.GetSystemMetrics(79)
        if width <= 0 or height <= 0:
            raise RuntimeError("无法读取桌面尺寸，已阻止鼠标输入")
        x = max(0, min(65535, round((point.x - left) * 65535 / max(width - 1, 1))))
        y = max(0, min(65535, round((point.y - top) * 65535 / max(height - 1, 1))))
        self._mouse(_MOUSE_MOVE | _MOUSE_ABSOLUTE | _MOUSE_VIRTUAL_DESKTOP, x, y)

    def move_relative(self, dx: int, dy: int) -> None:
        self._mouse(_MOUSE_MOVE, dx, dy)

    def cursor_position(self) -> Point:
        position = wintypes.POINT()
        if not self._user32.GetCursorPos(ctypes.byref(position)):
            raise OSError(ctypes.get_last_error(), "GetCursorPos failed")
        return Point(position.x, position.y)

    def client_to_screen(self, point: Point) -> Point:
        position = wintypes.POINT(point.x, point.y)
        if not self._expected_hwnd or not self._user32.ClientToScreen(self._expected_hwnd, ctypes.byref(position)):
            raise OSError(ctypes.get_last_error(), "无法转换游戏窗口点击坐标")
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
        self._send(self._keyboard_event(key, up=False))

    def key_up(self, key: str) -> None:
        self._send(self._keyboard_event(key, up=True), False)

    @staticmethod
    def _keyboard_event(key: str, up: bool) -> _Input:
        # Physical scan codes are used for every supported gameplay key, not
        # only modifiers. Games polling physical keys can miss virtual-key-only input.
        scan = _scan_code(key)
        flags = _KEY_UP if up else 0
        return _Input(1, _InputUnion(ki=_KeyboardInput(0, scan, flags | _KEY_SCANCODE, 0, 0)))


class WindowsClientInput(InputBackend):
    """Client-coordinate input for login/diagnostics, using the same foreground transport."""

    def __init__(self, hwnd_provider: Callable[[], int | None]) -> None:
        self._hwnd_provider = hwnd_provider
        self._backend: WindowsSendInput | None = None

    def _target(self) -> WindowsSendInput:
        hwnd = self._hwnd_provider()
        if not hwnd:
            raise ForegroundLostError("游戏窗口不存在，已阻止输入")
        if self._backend is None or self._backend._expected_hwnd != hwnd:
            self._backend = WindowsSendInput(hwnd)
        return self._backend

    def move_mouse(self, point: Point) -> None:
        backend = self._target()
        end = backend.client_to_screen(point)
        start = backend.cursor_position()
        route = curved_route((start.x, start.y), (end.x, end.y))
        duration = route_duration((start.x, start.y), route)
        for x, y in route:
            backend.move_mouse(Point(x, y))
            sleep(duration / len(route))

    def mouse_down(self, button: str = "left") -> None:
        self._target().mouse_down(button)

    def mouse_up(self, button: str = "left") -> None:
        if self._backend is not None:
            self._backend.mouse_up(button)

    def key_down(self, key: str) -> None:
        self._target().key_down(key)

    def key_up(self, key: str) -> None:
        if self._backend is not None:
            self._backend.key_up(key)


def _scan_code(key: str) -> int:
    codes = {"ESC": 0x01, "TAB": 0x0F, "ENTER": 0x1C, "ALT": 0x38, "SPACE": 0x39}
    for row, start in (("1234567890", 0x02), ("QWERTYUIOP", 0x10), ("ASDFGHJKL", 0x1E), ("ZXCVBNM", 0x2C)):
        codes.update({letter: start + index for index, letter in enumerate(row)})
    try:
        return codes[key.upper()]
    except KeyError as error:
        raise ValueError(f"unsupported key: {key}") from error


def _virtual_key(key: str) -> int:
    normalized = key.upper()
    named = {"ENTER": 0x0D, "ESC": 0x1B, "SPACE": 0x20, "TAB": 0x09, "ALT": 0x12}
    if normalized in named:
        return named[normalized]
    if len(normalized) == 1 and normalized in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789":
        return ord(normalized)
    raise ValueError(f"unsupported key: {key}")
