from __future__ import annotations

import asyncio
from types import SimpleNamespace

from app.engine import AutomationEngine
from config.schema import EngineConfig
from core.models import Rect
from window.base import WindowInfo
from window.windows import WindowsWindowManager


class FakeWin32Con:
    GWL_STYLE = -16
    GWL_EXSTYLE = -20
    WS_THICKFRAME = 0x00040000
    WS_CAPTION = 0x00C00000
    WS_BORDER = 0x00800000
    WS_SIZEBOX = WS_THICKFRAME
    WS_MAXIMIZEBOX = 0x00010000
    WS_EX_TOPMOST = 0x00000008
    HWND_TOPMOST = -1
    HWND_NOTOPMOST = -2
    SW_RESTORE = 9
    SW_SHOWNOACTIVATE = 4
    SWP_NOSIZE = 0x0001
    SWP_NOMOVE = 0x0002
    SWP_NOZORDER = 0x0004
    SWP_NOACTIVATE = 0x0010
    SWP_FRAMECHANGED = 0x0020


class FakeWin32Gui:
    def __init__(self, style: int, ex_style: int = 0) -> None:
        self.style = style
        self.ex_style = ex_style
        self.window_rect = (100, 200, 1400, 950)
        self.client_rect = (0, 0, 1280, 720)
        self.calls: list[tuple] = []

    def GetWindowLong(self, hwnd: int, index: int) -> int:
        return self.style if index == FakeWin32Con.GWL_STYLE else self.ex_style

    def SetWindowLong(self, hwnd: int, index: int, value: int) -> None:
        self.calls.append(("SetWindowLong", hwnd, index, value))
        if index == FakeWin32Con.GWL_STYLE:
            self.style = value
        else:
            self.ex_style = value

    def GetWindowRect(self, hwnd: int) -> tuple[int, int, int, int]:
        return self.window_rect

    def GetClientRect(self, hwnd: int) -> tuple[int, int, int, int]:
        if not self.style & (
            FakeWin32Con.WS_THICKFRAME | FakeWin32Con.WS_CAPTION | FakeWin32Con.WS_BORDER
        ):
            return (0, 0, self.window_rect[2] - self.window_rect[0], self.window_rect[3] - self.window_rect[1])
        return self.client_rect

    def IsIconic(self, hwnd: int) -> bool:
        return True

    def IsWindow(self, hwnd: int) -> bool:
        return True

    def ShowWindow(self, hwnd: int, command: int) -> None:
        self.calls.append(("ShowWindow", hwnd, command))

    def SetForegroundWindow(self, hwnd: int) -> None:
        self.calls.append(("SetForegroundWindow", hwnd))

    def BringWindowToTop(self, hwnd: int) -> None:
        self.calls.append(("BringWindowToTop", hwnd))

    def SetWindowPos(self, hwnd: int, insert_after, x: int, y: int, width: int, height: int, flags: int) -> None:
        self.calls.append(("SetWindowPos", hwnd, insert_after, x, y, width, height, flags))


def _manager(gui: FakeWin32Gui) -> WindowsWindowManager:
    manager = object.__new__(WindowsWindowManager)
    manager._window = None
    manager._user32 = None
    manager._kernel32 = None
    manager._style_snapshot = None
    manager._automation_style_applied = False
    manager.current_hwnd = lambda: 42  # type: ignore[method-assign]
    manager._load_pywin32 = lambda: (FakeWin32Con, gui)  # type: ignore[method-assign]
    return manager


def test_apply_style_uses_aalc_sequence_and_is_idempotent(monkeypatch) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    original_style = (
        FakeWin32Con.WS_THICKFRAME
        | FakeWin32Con.WS_CAPTION
        | FakeWin32Con.WS_BORDER
        | FakeWin32Con.WS_MAXIMIZEBOX
        | 0x10000000
    )
    gui = FakeWin32Gui(original_style)
    manager = _manager(gui)

    assert manager.apply_automation_style()
    first_calls = list(gui.calls)
    assert manager.apply_automation_style()
    assert gui.calls == first_calls

    assert first_calls[0] == ("ShowWindow", 42, FakeWin32Con.SW_RESTORE)
    assert first_calls[1] == ("BringWindowToTop", 42)
    assert first_calls[2] == ("SetForegroundWindow", 42)
    assert first_calls[3][0:3] == ("SetWindowPos", 42, FakeWin32Con.HWND_TOPMOST)
    assert first_calls[4][0:3] == ("SetWindowLong", 42, FakeWin32Con.GWL_STYLE)
    assert gui.style == 0x10000000
    assert first_calls[5][-1] & FakeWin32Con.SWP_FRAMECHANGED
    assert first_calls[6][5:7] == (1280, 720)


def test_restore_style_restores_exact_original_state(monkeypatch) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    original_style = FakeWin32Con.WS_CAPTION | FakeWin32Con.WS_THICKFRAME | 0x10000000
    original_ex_style = FakeWin32Con.WS_EX_TOPMOST | 0x00040000
    gui = FakeWin32Gui(original_style, original_ex_style)
    manager = _manager(gui)

    manager.apply_automation_style()
    assert manager.restore_automation_style()
    assert gui.style == original_style
    assert gui.ex_style == original_ex_style
    assert gui.calls[-1][0:7] == (
        "SetWindowPos",
        42,
        FakeWin32Con.HWND_TOPMOST,
        100,
        200,
        1300,
        750,
    )
    assert manager._style_snapshot is None
    assert not manager._automation_style_applied
    call_count = len(gui.calls)
    assert manager.restore_automation_style()
    assert len(gui.calls) == call_count


def test_engine_applies_and_restores_style_during_lifecycle() -> None:
    class Writer:
        async def send(self, message: dict) -> None:
            pass

    class Reader:
        async def messages(self):
            yield {"type": "shutdown"}

    class Window:
        applied = 0
        restored = 0

        def find_window(self) -> WindowInfo:
            return WindowInfo(42, "Game", "Class", 1, Rect(0, 0, 1920, 1080), 1.0)

        def apply_automation_style(self) -> bool:
            self.applied += 1
            return True

        def restore_automation_style(self) -> bool:
            self.restored += 1
            return True

    engine = AutomationEngine(EngineConfig())
    window = Window()
    engine.writer = Writer()
    engine.reader = Reader()
    engine.context.window = window
    engine.context.input = SimpleNamespace(release_all=lambda: None)

    asyncio.run(engine.run())

    assert window.applied >= 1
    assert window.restored == 1
