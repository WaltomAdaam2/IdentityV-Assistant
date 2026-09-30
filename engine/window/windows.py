from __future__ import annotations

import ctypes
import sys
from ctypes import wintypes
from pathlib import Path
from typing import Any

from config.schema import WindowConfig
from core.models import Rect
from window.base import WindowInfo, WindowManager


_window_enum_proc = getattr(ctypes, "WINFUNCTYPE", ctypes.CFUNCTYPE)(
    wintypes.BOOL,
    wintypes.HWND,
    wintypes.LPARAM,
)


class WindowsWindowManager(WindowManager):
    """Win32 implementation using only visible top-level windows."""

    def __init__(self, config: WindowConfig) -> None:
        self.config = config
        self._window: WindowInfo | None = None
        self._original_window: tuple[int, int, int, tuple[int, int, int, int]] | None = None
        self._user32: Any | None = None
        self._kernel32: Any | None = None
        if sys.platform == "win32":
            self._configure_winapi()

    def _configure_winapi(self) -> None:
        self._user32 = ctypes.WinDLL("user32", use_last_error=True)
        self._kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        user32, kernel32 = self._user32, self._kernel32

        user32.EnumWindows.argtypes = [_window_enum_proc, wintypes.LPARAM]
        user32.EnumWindows.restype = wintypes.BOOL
        user32.IsWindowVisible.argtypes = [wintypes.HWND]
        user32.IsWindowVisible.restype = wintypes.BOOL
        user32.GetWindowTextLengthW.argtypes = [wintypes.HWND]
        user32.GetWindowTextLengthW.restype = ctypes.c_int
        user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
        user32.GetWindowTextW.restype = ctypes.c_int
        user32.GetClassNameW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
        user32.GetClassNameW.restype = ctypes.c_int
        user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
        user32.GetWindowThreadProcessId.restype = wintypes.DWORD
        user32.GetClientRect.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.RECT)]
        user32.GetClientRect.restype = wintypes.BOOL
        user32.ClientToScreen.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.POINT)]
        user32.ClientToScreen.restype = wintypes.BOOL
        user32.IsIconic.argtypes = [wintypes.HWND]
        user32.IsIconic.restype = wintypes.BOOL
        user32.GetForegroundWindow.argtypes = []
        user32.GetForegroundWindow.restype = wintypes.HWND
        user32.IsWindow.argtypes = [wintypes.HWND]
        user32.IsWindow.restype = wintypes.BOOL
        user32.SetForegroundWindow.argtypes = [wintypes.HWND]
        user32.SetForegroundWindow.restype = wintypes.BOOL
        user32.SendMessageTimeoutW.argtypes = [
            wintypes.HWND,
            wintypes.UINT,
            wintypes.WPARAM,
            wintypes.LPARAM,
            wintypes.UINT,
            wintypes.UINT,
            ctypes.POINTER(ctypes.c_size_t),
        ]
        user32.SendMessageTimeoutW.restype = ctypes.c_size_t

        get_dpi = getattr(user32, "GetDpiForWindow", None)
        if get_dpi:
            get_dpi.argtypes = [wintypes.HWND]
            get_dpi.restype = wintypes.UINT
        set_context = getattr(user32, "SetProcessDpiAwarenessContext", None)
        if set_context:
            set_context.argtypes = [ctypes.c_void_p]
            set_context.restype = wintypes.BOOL

        kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel32.OpenProcess.restype = wintypes.HANDLE
        kernel32.QueryFullProcessImageNameW.argtypes = [
            wintypes.HANDLE,
            wintypes.DWORD,
            wintypes.LPWSTR,
            ctypes.POINTER(wintypes.DWORD),
        ]
        kernel32.QueryFullProcessImageNameW.restype = wintypes.BOOL
        kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel32.CloseHandle.restype = wintypes.BOOL

    def find_window(self) -> WindowInfo | None:
        if sys.platform != "win32" or self._user32 is None:
            return None
        user32 = self._user32
        self._enable_dpi_awareness()
        matches: list[WindowInfo] = []

        @_window_enum_proc
        def callback(hwnd: int, _: int) -> bool:
            if not user32.IsWindowVisible(hwnd):
                return True
            length = user32.GetWindowTextLengthW(hwnd)
            title_buffer = ctypes.create_unicode_buffer(length + 1)
            user32.GetWindowTextW(hwnd, title_buffer, len(title_buffer))
            class_buffer = ctypes.create_unicode_buffer(256)
            user32.GetClassNameW(hwnd, class_buffer, len(class_buffer))
            title, class_name = title_buffer.value, class_buffer.value
            pid = wintypes.DWORD()
            user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
            process_name = self._process_name(pid.value)
            reasons = self._match_reasons(title, class_name, process_name)
            if not self._is_configured_match(reasons):
                return True
            rect = self._client_rect(hwnd)
            if rect is not None and rect.width > 0 and rect.height > 0:
                matches.append(
                    WindowInfo(
                        hwnd=int(hwnd),
                        title=title,
                        class_name=class_name,
                        process_id=pid.value,
                        client_rect=rect,
                        dpi_scale=self._dpi_scale(hwnd),
                        process_name=process_name,
                        is_minimized=bool(user32.IsIconic(hwnd)),
                        is_foreground=user32.GetForegroundWindow() == hwnd,
                        is_responding=self._is_responding(hwnd),
                        match_reasons=reasons,
                    )
                )
            return True

        if not user32.EnumWindows(callback, 0):
            self._window = None
            return None
        matches.sort(key=self._window_rank)
        self._window = matches[0] if matches else None
        return self._window

    def _match_reasons(self, title: str, class_name: str, process_name: str) -> tuple[str, ...]:
        reasons: list[str] = []
        if any(needle.casefold() in title.casefold() for needle in self.config.title_contains):
            reasons.append("title")
        if self.config.class_names and class_name.casefold() in {
            name.casefold() for name in self.config.class_names
        }:
            reasons.append("class")
        if process_name.casefold() in {name.casefold() for name in self.config.process_names}:
            reasons.append("process")
        return tuple(reasons)

    def _is_configured_match(self, reasons: tuple[str, ...]) -> bool:
        if self.config.process_names:
            return "process" in reasons
        return bool(reasons)

    @staticmethod
    def _window_rank(window: WindowInfo) -> tuple[bool, int, bool, int, int]:
        area = window.client_rect.width * window.client_rect.height
        return (
            window.is_minimized,
            -len(window.match_reasons),
            not window.is_foreground,
            -area,
            window.hwnd,
        )

    def _enable_dpi_awareness(self) -> None:
        if self._user32 is None:
            return
        set_context = getattr(self._user32, "SetProcessDpiAwarenessContext", None)
        if set_context:
            try:
                set_context(ctypes.c_void_p(-4))
            except OSError:
                pass

    def _process_name(self, pid: int) -> str:
        if sys.platform != "win32" or self._kernel32 is None:
            return ""
        process = self._kernel32.OpenProcess(0x1000, False, pid)
        if not process:
            return ""
        try:
            buffer = ctypes.create_unicode_buffer(32768)
            length = wintypes.DWORD(len(buffer))
            if self._kernel32.QueryFullProcessImageNameW(
                process,
                0,
                buffer,
                ctypes.byref(length),
            ):
                return Path(buffer.value).name
            return ""
        finally:
            self._kernel32.CloseHandle(process)

    def _client_rect(self, hwnd: int) -> Rect | None:
        if sys.platform != "win32" or self._user32 is None:
            return None
        rect = wintypes.RECT()
        if not self._user32.GetClientRect(hwnd, ctypes.byref(rect)):
            return None
        point = wintypes.POINT(rect.left, rect.top)
        if not self._user32.ClientToScreen(hwnd, ctypes.byref(point)):
            return None
        return Rect(point.x, point.y, rect.right - rect.left, rect.bottom - rect.top)

    def _dpi_scale(self, hwnd: int) -> float:
        if self._user32 is None:
            return 1.0
        get_dpi = getattr(self._user32, "GetDpiForWindow", None)
        return (get_dpi(hwnd) / 96.0) if get_dpi else 1.0

    def get_client_rect(self) -> Rect | None:
        return self._client_rect(self._window.hwnd) if self._window else None

    def is_alive(self) -> bool:
        return bool(self._window and self._user32 and self._user32.IsWindow(self._window.hwnd))

    def current_hwnd(self) -> int | None:
        if not self.is_alive():
            self.find_window()
        return self._window.hwnd if self._window else None

    def is_minimized(self) -> bool:
        return bool(self._window and self._user32 and self._user32.IsIconic(self._window.hwnd))

    def is_responding(self) -> bool:
        return bool(self._window and self._is_responding(self._window.hwnd))

    def _is_responding(self, hwnd: int) -> bool:
        if self._user32 is None:
            return False
        result = ctypes.c_size_t()
        return bool(
            self._user32.SendMessageTimeoutW(
                hwnd,
                0,
                0,
                0,
                0x0001 | 0x0002,
                250,
                ctypes.byref(result),
            )
        )

    def bring_to_front(self) -> None:
        if self._window and self._user32:
            self._user32.SetForegroundWindow(self._window.hwnd)

    def get_dpi_scale(self) -> float:
        return self._dpi_scale(self._window.hwnd) if self._window else 1.0

    def prepare_background_mode(self) -> bool:
        if not self.config.background_mode:
            return True
        hwnd = self.current_hwnd()
        if not hwnd:
            return False
        import win32api
        import win32con
        import win32gui

        if self._original_window is None or self._original_window[0] != hwnd:
            self._original_window = (
                hwnd,
                win32gui.GetWindowLong(hwnd, win32con.GWL_STYLE),
                win32gui.GetWindowLong(hwnd, win32con.GWL_EXSTYLE),
                win32gui.GetWindowRect(hwnd),
            )
        if win32gui.IsIconic(hwnd):
            win32gui.ShowWindow(hwnd, win32con.SW_SHOWNOACTIVATE)

        style = win32gui.GetWindowLong(hwnd, win32con.GWL_STYLE)
        style &= ~(
            win32con.WS_CAPTION
            | win32con.WS_BORDER
            | win32con.WS_THICKFRAME
            | win32con.WS_SIZEBOX
            | win32con.WS_MAXIMIZEBOX
            | win32con.WS_MINIMIZEBOX
            | win32con.WS_SYSMENU
        )
        style |= win32con.WS_POPUP | win32con.WS_VISIBLE
        ex_style = win32gui.GetWindowLong(hwnd, win32con.GWL_EXSTYLE)
        ex_style &= ~(
            win32con.WS_EX_DLGMODALFRAME
            | win32con.WS_EX_CLIENTEDGE
            | win32con.WS_EX_STATICEDGE
            | win32con.WS_EX_WINDOWEDGE
        )
        win32gui.SetWindowLong(hwnd, win32con.GWL_STYLE, style)
        win32gui.SetWindowLong(hwnd, win32con.GWL_EXSTYLE, ex_style)
        win32gui.SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            win32con.SWP_NOMOVE
            | win32con.SWP_NOSIZE
            | win32con.SWP_NOZORDER
            | win32con.SWP_NOACTIVATE
            | win32con.SWP_FRAMECHANGED,
        )
        if win32gui.GetWindowLong(hwnd, win32con.GWL_STYLE) & win32con.WS_CAPTION:
            raise RuntimeError("the game restored its title bar")

        width, height = map(int, self.config.resolution.split("x"))
        monitor = win32api.GetMonitorInfo(
            win32api.MonitorFromWindow(hwnd, win32con.MONITOR_DEFAULTTONEAREST)
        )["Monitor"]
        left, top, right, bottom = monitor
        positions = {
            "center": (left + (right - left - width) // 2, top + (bottom - top - height) // 2),
            "left_top": (left, top),
            "right_top": (right - width, top),
            "left_bottom": (left, bottom - height),
            "right_bottom": (right - width, bottom - height),
        }
        flags = win32con.SWP_NOZORDER | win32con.SWP_NOACTIVATE
        x, y = positions.get(self.config.position, (0, 0))
        if self.config.position == "free":
            flags |= win32con.SWP_NOMOVE
        win32gui.SetWindowPos(hwnd, None, x, y, width, height, flags)
        self.find_window()
        return True

    def restore_window_mode(self) -> bool:
        if self._original_window is None:
            return True
        hwnd = self.current_hwnd()
        if not hwnd:
            self._original_window = None
            return False
        import win32con
        import win32gui

        original_hwnd, style, ex_style, rect = self._original_window
        if self._user32 and not self._user32.IsWindow(original_hwnd):
            self._original_window = None
            return False
        hwnd = original_hwnd
        win32gui.SetWindowLong(hwnd, win32con.GWL_STYLE, style)
        win32gui.SetWindowLong(hwnd, win32con.GWL_EXSTYLE, ex_style)
        win32gui.SetWindowPos(
            hwnd,
            win32con.HWND_NOTOPMOST,
            rect[0],
            rect[1],
            rect[2] - rect[0],
            rect[3] - rect[1],
            win32con.SWP_NOACTIVATE | win32con.SWP_FRAMECHANGED,
        )
        win32gui.ShowWindow(hwnd, win32con.SW_SHOWNOACTIVATE)
        self._original_window = None
        self.find_window()
        return True
