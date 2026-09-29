from __future__ import annotations

import ctypes
import sys
from pathlib import Path
from ctypes import wintypes

from config.schema import WindowConfig
from core.models import Rect
from window.base import WindowInfo, WindowManager


class WindowsWindowManager(WindowManager):
    """Win32 implementation using only visible top-level windows."""

    def __init__(self, config: WindowConfig) -> None:
        self.config = config
        self._window: WindowInfo | None = None

    def find_window(self) -> WindowInfo | None:
        if sys.platform != "win32":
            return None
        user32 = ctypes.windll.user32
        self._enable_dpi_awareness(user32)
        matches: list[WindowInfo] = []

        @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
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
            if not reasons:
                return True
            rect = self._client_rect(hwnd)
            if rect is not None and rect.width > 0 and rect.height > 0:
                matches.append(WindowInfo(
                    hwnd=hwnd,
                    title=title,
                    class_name=class_name,
                    process_id=pid.value,
                    client_rect=rect,
                    dpi_scale=self._dpi_scale(hwnd),
                    process_name=process_name,
                    is_minimized=bool(user32.IsIconic(hwnd)),
                    is_foreground=user32.GetForegroundWindow() == hwnd,
                    match_reasons=reasons,
                ))
            return True

        user32.EnumWindows(callback, 0)
        matches.sort(key=self._window_rank)
        self._window = matches[0] if matches else None
        return self._window

    def _match_reasons(self, title: str, class_name: str, process_name: str) -> tuple[str, ...]:
        reasons: list[str] = []
        if any(needle.casefold() in title.casefold() for needle in self.config.title_contains):
            reasons.append("title")
        if self.config.class_names and class_name.casefold() in {name.casefold() for name in self.config.class_names}:
            reasons.append("class")
        if process_name.casefold() in {name.casefold() for name in self.config.process_names}:
            reasons.append("process")
        return tuple(reasons)

    @staticmethod
    def _window_rank(window: WindowInfo) -> tuple[bool, bool, int, int]:
        area = window.client_rect.width * window.client_rect.height
        return window.is_minimized, not window.is_foreground, -area, window.hwnd

    @staticmethod
    def _enable_dpi_awareness(user32: object) -> None:
        set_context = getattr(user32, "SetProcessDpiAwarenessContext", None)
        if set_context:
            try:
                set_context(ctypes.c_void_p(-4))
            except OSError:
                pass

    @staticmethod
    def _process_name(pid: int) -> str:
        if sys.platform != "win32":
            return ""
        process = ctypes.windll.kernel32.OpenProcess(0x1000, False, pid)
        if not process:
            return ""
        try:
            buffer = ctypes.create_unicode_buffer(32768)
            length = wintypes.DWORD(len(buffer))
            if ctypes.windll.kernel32.QueryFullProcessImageNameW(process, 0, buffer, ctypes.byref(length)):
                return Path(buffer.value).name
            return ""
        finally:
            ctypes.windll.kernel32.CloseHandle(process)

    def _client_rect(self, hwnd: int) -> Rect | None:
        if sys.platform != "win32":
            return None
        rect = wintypes.RECT()
        if not ctypes.windll.user32.GetClientRect(hwnd, ctypes.byref(rect)):
            return None
        point = wintypes.POINT(rect.left, rect.top)
        if not ctypes.windll.user32.ClientToScreen(hwnd, ctypes.byref(point)):
            return None
        return Rect(point.x, point.y, rect.right - rect.left, rect.bottom - rect.top)

    def _dpi_scale(self, hwnd: int) -> float:
        get_dpi = getattr(ctypes.windll.user32, "GetDpiForWindow", None)
        return (get_dpi(hwnd) / 96.0) if get_dpi else 1.0

    def get_client_rect(self) -> Rect | None:
        return self._client_rect(self._window.hwnd) if self._window else None

    def is_alive(self) -> bool:
        return bool(self._window and sys.platform == "win32" and ctypes.windll.user32.IsWindow(self._window.hwnd))

    def is_minimized(self) -> bool:
        return bool(self._window and sys.platform == "win32" and ctypes.windll.user32.IsIconic(self._window.hwnd))

    def bring_to_front(self) -> None:
        if self._window and sys.platform == "win32":
            ctypes.windll.user32.SetForegroundWindow(self._window.hwnd)

    def get_dpi_scale(self) -> float:
        return self._dpi_scale(self._window.hwnd) if self._window else 1.0
