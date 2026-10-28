"""Reversible automation window layout using ordinary Win32 window APIs."""

import logging
from dataclasses import dataclass


logger = logging.getLogger(__name__)


@dataclass(frozen=True)
class _WindowSnapshot:
    hwnd: int
    style: int
    exstyle: int
    rect: tuple[int, int, int, int]


class WindowStyleController:
    """Keep the original window state until it has been restored successfully.

    The optional module arguments provide the same API as pywin32 and permit
    offline tests without opening or changing a real window.
    """

    def __init__(self, *, gui=None, api=None, constants=None):
        self._gui = gui
        self._api = api
        self._constants = constants
        self._snapshot: _WindowSnapshot | None = None
        self._applied: tuple[int, str, str] | None = None

    def _load_modules(self) -> None:
        if self._gui is None:
            import win32gui

            self._gui = win32gui
        if self._api is None:
            import win32api

            self._api = win32api
        if self._constants is None:
            import win32con

            self._constants = win32con

    def apply(self, hwnd: int, resolution: str, position: str) -> bool:
        """Apply a borderless, topmost layout with the requested client size."""
        try:
            width, height = (int(value) for value in resolution.split("x"))
            if width <= 0 or height <= 0 or position not in {
                "center", "unrestricted", "top_left", "top_right", "bottom_left", "bottom_right"
            }:
                return False
            self._load_modules()
            gui, api, c = self._gui, self._api, self._constants
            if not gui.IsWindow(hwnd):
                return False
            if self._snapshot is not None and self._snapshot.hwnd != hwnd:
                if not self.restore():
                    return False
            key = (hwnd, resolution, position)
            if self._applied == key and not gui.IsIconic(hwnd):
                return True
            if self._snapshot is None:
                self._snapshot = _WindowSnapshot(
                    hwnd,
                    gui.GetWindowLong(hwnd, c.GWL_STYLE),
                    gui.GetWindowLong(hwnd, c.GWL_EXSTYLE),
                    tuple(gui.GetWindowRect(hwnd)),
                )
            self._applied = None
            snapshot = self._snapshot
            if gui.IsIconic(hwnd) or gui.GetWindowLong(hwnd, c.GWL_STYLE) & c.WS_MAXIMIZE:
                gui.ShowWindow(hwnd, c.SW_RESTORE)

            style = snapshot.style & ~(
                c.WS_CAPTION | c.WS_THICKFRAME | c.WS_BORDER | c.WS_MAXIMIZEBOX
                | c.WS_MINIMIZE | c.WS_MAXIMIZE
            )
            exstyle = snapshot.exstyle & ~(
                c.WS_EX_CLIENTEDGE | c.WS_EX_WINDOWEDGE | c.WS_EX_DLGMODALFRAME | c.WS_EX_STATICEDGE
            )
            gui.SetWindowLong(hwnd, c.GWL_STYLE, style)
            gui.SetWindowLong(hwnd, c.GWL_EXSTYLE, exstyle)
            # Commit the frame change before measuring its actual client area.
            gui.SetWindowPos(
                hwnd, 0, 0, 0, 0, 0,
                c.SWP_FRAMECHANGED | c.SWP_NOMOVE | c.SWP_NOSIZE | c.SWP_NOZORDER | c.SWP_NOACTIVATE,
            )
            left, top, right, bottom = gui.GetWindowRect(hwnd)
            client_left, client_top, client_right, client_bottom = gui.GetClientRect(hwnd)
            outer_width = width + (right - left) - (client_right - client_left)
            outer_height = height + (bottom - top) - (client_bottom - client_top)
            monitor = api.MonitorFromWindow(hwnd, c.MONITOR_DEFAULTTONEAREST)
            work_left, work_top, work_right, work_bottom = api.GetMonitorInfo(monitor)["Work"]
            max_left = max(work_left, work_right - outer_width)
            max_top = max(work_top, work_bottom - outer_height)
            if position == "center":
                left = (work_left + work_right - outer_width) // 2
                top = (work_top + work_bottom - outer_height) // 2
            elif position != "unrestricted":
                left = max_left if position.endswith("right") else work_left
                top = max_top if position.startswith("bottom") else work_top
            if position != "unrestricted":
                left = max(work_left, min(left, max_left))
                top = max(work_top, min(top, max_top))
            gui.SetWindowPos(
                hwnd, c.HWND_TOPMOST, left, top, outer_width, outer_height,
                c.SWP_NOACTIVATE | c.SWP_SHOWWINDOW,
            )
            client_left, client_top, client_right, client_bottom = gui.GetClientRect(hwnd)
            if (client_right - client_left, client_bottom - client_top) != (width, height):
                return False
            # Windows can deny foreground permission; the input layer retries it.
            try:
                gui.SetForegroundWindow(hwnd)
            except Exception:
                logger.debug("Windows declined foreground activation for %s", hwnd, exc_info=True)
            self._applied = key
            return True
        except Exception:
            logger.warning("Could not apply automation window layout", exc_info=True)
            return False

    def restore(self) -> bool:
        """Restore the saved style, extended style, rectangle and topmost state."""
        snapshot = self._snapshot
        if snapshot is None:
            return True
        self._applied = None
        try:
            self._load_modules()
            gui, c = self._gui, self._constants
            if not gui.IsWindow(snapshot.hwnd):
                # A destroyed window has no remaining state to restore.
                self._snapshot = None
                return True
            gui.SetWindowLong(snapshot.hwnd, c.GWL_STYLE, snapshot.style)
            gui.SetWindowLong(snapshot.hwnd, c.GWL_EXSTYLE, snapshot.exstyle)
            left, top, right, bottom = snapshot.rect
            insert_after = c.HWND_TOPMOST if snapshot.exstyle & c.WS_EX_TOPMOST else c.HWND_NOTOPMOST
            gui.SetWindowPos(
                snapshot.hwnd, insert_after, left, top, right - left, bottom - top,
                c.SWP_FRAMECHANGED | c.SWP_NOACTIVATE,
            )
        except Exception:
            logger.warning("Could not restore automation window layout", exc_info=True)
            return False
        self._snapshot = None
        return True
