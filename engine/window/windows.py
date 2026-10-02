from __future__ import annotations

import ctypes
import importlib
import sys
from dataclasses import dataclass
from ctypes import wintypes
from pathlib import Path
from time import sleep
from typing import Any

from config.schema import WindowConfig
from core.models import Rect
from window.base import WindowInfo, WindowManager


_window_enum_proc = getattr(ctypes, "WINFUNCTYPE", ctypes.CFUNCTYPE)(
    wintypes.BOOL,
    wintypes.HWND,
    wintypes.LPARAM,
)


@dataclass(frozen=True)
class _WindowStyleSnapshot:
    hwnd: int
    style: int
    ex_style: int
    window_rect: tuple[int, int, int, int]
    client_size: tuple[int, int]


class WindowsWindowManager(WindowManager):
    """Win32 implementation using only visible top-level windows."""

    def __init__(self, config: WindowConfig) -> None:
        self.config = config
        self._window: WindowInfo | None = None
        self._user32: Any | None = None
        self._kernel32: Any | None = None
        self._style_snapshot: _WindowStyleSnapshot | None = None
        self._automation_style_applied = False
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
        user32.ShowWindow.argtypes = [wintypes.HWND, ctypes.c_int]
        user32.ShowWindow.restype = wintypes.BOOL
        user32.BringWindowToTop.argtypes = [wintypes.HWND]
        user32.BringWindowToTop.restype = wintypes.BOOL
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
            if self._user32.IsIconic(self._window.hwnd):
                self._user32.ShowWindow(self._window.hwnd, 9)
            self._user32.BringWindowToTop(self._window.hwnd)
            self._user32.SetForegroundWindow(self._window.hwnd)

    @staticmethod
    def _load_pywin32() -> tuple[Any, Any]:
        """Load the Win32 modules only when window styling is requested."""

        return importlib.import_module("win32con"), importlib.import_module("win32gui")

    def apply_automation_style(self) -> bool:
        """Restore, foreground, crop, and pin the current game window.

        The style mutation sequence is adapted from AALC's ``Screen.reduce_miscontact``
        and ``Screen.adjust_win_size`` implementation. See THIRD_PARTY_NOTICES.md.
        """

        hwnd = self.current_hwnd()
        if not hwnd:
            return False

        win32con, win32gui = self._load_pywin32()
        if self._style_snapshot and self._style_snapshot.hwnd != hwnd:
            self.restore_automation_style()

        if self._style_snapshot is None:
            client_rect = win32gui.GetClientRect(hwnd)
            self._style_snapshot = _WindowStyleSnapshot(
                hwnd=hwnd,
                style=win32gui.GetWindowLong(hwnd, win32con.GWL_STYLE),
                ex_style=win32gui.GetWindowLong(hwnd, win32con.GWL_EXSTYLE),
                window_rect=tuple(win32gui.GetWindowRect(hwnd)),
                client_size=(client_rect[2] - client_rect[0], client_rect[3] - client_rect[1]),
            )

        if self._automation_style_applied:
            return True

        # AALC: restore the window before changing its frame and foreground state.
        if win32gui.IsIconic(hwnd):
            win32gui.ShowWindow(hwnd, win32con.SW_RESTORE)
        win32gui.SetForegroundWindow(hwnd)

        # AALC Screen.reduce_miscontact: keep this call order and flag set intact.
        win32gui.SetWindowPos(
            hwnd,
            win32con.HWND_TOPMOST,
            0,
            0,
            0,
            0,
            win32con.SWP_NOMOVE | win32con.SWP_NOSIZE,
        )
        style = win32gui.GetWindowLong(hwnd, win32con.GWL_STYLE)
        style &= ~win32con.WS_THICKFRAME
        style &= ~win32con.WS_CAPTION
        style &= ~win32con.WS_BORDER
        style &= ~(win32con.WS_SIZEBOX | win32con.WS_MAXIMIZEBOX)
        win32gui.SetWindowLong(hwnd, win32con.GWL_STYLE, style)
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
            | win32con.SWP_FRAMECHANGED,
        )
        sleep(0.1)

        # AALC Screen.adjust_win_size: retain the original client dimensions after
        # removing the non-client frame instead of changing the game's render size.
        client_width, client_height = self._style_snapshot.client_size
        window_rect = win32gui.GetWindowRect(hwnd)
        client_rect = win32gui.GetClientRect(hwnd)
        window_width = window_rect[2] - window_rect[0]
        window_height = window_rect[3] - window_rect[1]
        current_client_width = client_rect[2] - client_rect[0]
        current_client_height = client_rect[3] - client_rect[1]
        required_window_width = client_width + (window_width - current_client_width)
        required_window_height = client_height + (window_height - current_client_height)
        win32gui.SetWindowPos(
            hwnd,
            None,
            0,
            0,
            required_window_width,
            required_window_height,
            win32con.SWP_NOZORDER | win32con.SWP_NOMOVE,
        )
        self._automation_style_applied = True
        return True

    def restore_automation_style(self) -> bool:
        """Restore the window state captured before automation styling."""

        snapshot = self._style_snapshot
        if snapshot is None:
            return True

        win32con, win32gui = self._load_pywin32()
        if not win32gui.IsWindow(snapshot.hwnd):
            self._style_snapshot = None
            self._automation_style_applied = False
            return False

        hwnd = snapshot.hwnd
        # AALC Screen.reset_win: restore the frame, clear topmost, and refresh it.
        win32gui.SetWindowLong(hwnd, win32con.GWL_STYLE, snapshot.style)
        win32gui.SetWindowLong(hwnd, win32con.GWL_EXSTYLE, snapshot.ex_style)
        win32gui.SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            win32con.SWP_FRAMECHANGED
            | win32con.SWP_NOMOVE
            | win32con.SWP_NOSIZE
            | win32con.SWP_NOZORDER
            | win32con.SWP_NOACTIVATE,
        )
        win32gui.ShowWindow(hwnd, win32con.SW_SHOWNOACTIVATE)
        win32gui.SetWindowPos(
            hwnd,
            win32con.HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            win32con.SWP_NOMOVE | win32con.SWP_NOSIZE | win32con.SWP_NOACTIVATE,
        )

        left, top, right, bottom = snapshot.window_rect
        win32gui.SetWindowPos(
            hwnd,
            win32con.HWND_TOPMOST if snapshot.ex_style & win32con.WS_EX_TOPMOST else win32con.HWND_NOTOPMOST,
            left,
            top,
            right - left,
            bottom - top,
            win32con.SWP_NOACTIVATE | win32con.SWP_FRAMECHANGED,
        )
        self._style_snapshot = None
        self._automation_style_applied = False
        return True

    def get_dpi_scale(self) -> float:
        return self._dpi_scale(self._window.hwnd) if self._window else 1.0

