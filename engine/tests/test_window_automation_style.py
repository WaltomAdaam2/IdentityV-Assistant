from __future__ import annotations

import asyncio
from types import SimpleNamespace

import pytest

from app.engine import AutomationEngine
from config.schema import EngineConfig, WindowConfig
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
    MONITOR_DEFAULTTONEAREST = 2


class FakeWin32Gui:
    def __init__(self, style: int, ex_style: int = 0) -> None:
        self.style = style
        self.ex_style = ex_style
        self.window_rect = (100, 200, 1400, 950)
        self.client_rect = (0, 0, 1280, 720)
        self.work_area = (0, 0, 1920, 1040)
        self.remaining_frame = (0, 0)
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
            return (
                0,
                0,
                self.window_rect[2] - self.window_rect[0] - self.remaining_frame[0],
                self.window_rect[3] - self.window_rect[1] - self.remaining_frame[1],
            )
        return self.client_rect

    def MonitorFromWindow(self, hwnd: int, flags: int) -> int:
        assert hwnd == 42
        assert flags == FakeWin32Con.MONITOR_DEFAULTTONEAREST
        return 1

    def GetMonitorInfo(self, monitor: int) -> dict:
        assert monitor == 1
        return {"Work": self.work_area}

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
        left, top, right, bottom = self.window_rect
        if flags & FakeWin32Con.SWP_NOMOVE:
            x, y = left, top
        if flags & FakeWin32Con.SWP_NOSIZE:
            width, height = right - left, bottom - top
        self.window_rect = (x, y, x + width, y + height)


def _manager(gui: FakeWin32Gui) -> WindowsWindowManager:
    manager = object.__new__(WindowsWindowManager)
    manager.config = WindowConfig()
    manager._window = None
    manager._user32 = None
    manager._kernel32 = None
    manager._style_snapshot = None
    manager._automation_style_applied = False
    manager._applied_window_settings = None
    manager.current_hwnd = lambda: 42  # type: ignore[method-assign]
    manager._load_pywin32 = lambda: (FakeWin32Con, gui, gui)  # type: ignore[method-assign]
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
    assert first_calls[6][3:7] == (320, 0, 1600, 900)
    assert gui.GetClientRect(42) == (0, 0, 1600, 900)


def test_apply_style_uses_monitor_work_area_and_remaining_frame(monkeypatch) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    gui = FakeWin32Gui(FakeWin32Con.WS_CAPTION | FakeWin32Con.WS_THICKFRAME)
    gui.work_area = (-1920, -200, 0, 840)
    gui.remaining_frame = (8, 12)
    manager = _manager(gui)

    assert manager.apply_automation_style()

    assert gui.GetClientRect(42) == (0, 0, 1600, 900)
    assert gui.window_rect == (-1608, -200, 0, 712)


@pytest.mark.parametrize("resolution", ["2560x1440", "1920x1080", "1600x900", "1280x720"])
@pytest.mark.parametrize(
    ("position", "horizontal", "vertical"),
    [
        ("center", 0.5, 0.5),
        ("top_left", 0, 0),
        ("top_right", 1, 0),
        ("bottom_left", 0, 1),
        ("bottom_right", 1, 1),
    ],
)
def test_configured_resolution_and_position(monkeypatch, resolution, position, horizontal, vertical) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    gui = FakeWin32Gui(FakeWin32Con.WS_CAPTION | FakeWin32Con.WS_THICKFRAME)
    gui.work_area = (-3840, -1800, 0, 320)
    gui.remaining_frame = (8, 12)
    manager = _manager(gui)
    manager.config = WindowConfig(resolution=resolution, position=position)
    width, height = map(int, resolution.split("x"))

    assert manager.apply_automation_style()

    left = -3840 + int((3840 - width - 8) * horizontal)
    top = -1800 + int((2120 - height - 12) * vertical)
    assert gui.GetClientRect(42) == (0, 0, width, height)
    assert gui.window_rect == (left, top, left + width + 8, top + height + 12)
    first_calls = list(gui.calls)
    assert manager.apply_automation_style()
    assert gui.calls == first_calls


@pytest.mark.parametrize("resolution", ["2560x1440", "1920x1080", "1600x900", "1280x720"])
def test_unrestricted_resize_retains_current_position(monkeypatch, resolution) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    gui = FakeWin32Gui(FakeWin32Con.WS_CAPTION | FakeWin32Con.WS_THICKFRAME)
    manager = _manager(gui)
    manager.config = WindowConfig(resolution=resolution, position="unrestricted")
    width, height = map(int, resolution.split("x"))

    assert manager.apply_automation_style()

    assert gui.window_rect == (100, 200, 100 + width, 200 + height)
    assert gui.calls[-1][-1] & FakeWin32Con.SWP_NOMOVE
    # A manual move remains unrestricted on later heartbeats.
    gui.window_rect = (-250, 80, -250 + width, 80 + height)
    first_calls = list(gui.calls)
    assert manager.apply_automation_style()
    assert gui.calls == first_calls
    assert gui.window_rect == (-250, 80, -250 + width, 80 + height)


@pytest.mark.parametrize("position", ["center", "top_left", "top_right", "bottom_left", "bottom_right"])
def test_oversized_window_keeps_top_left_reachable(monkeypatch, position) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    gui = FakeWin32Gui(FakeWin32Con.WS_CAPTION)
    gui.work_area = (-1920, -200, 0, 840)
    manager = _manager(gui)
    manager.config = WindowConfig(resolution="2560x1440", position=position)

    assert manager.apply_automation_style()

    assert gui.window_rect == (-1920, -200, 640, 1240)
    assert gui.GetClientRect(42) == (0, 0, 2560, 1440)


def test_setting_changes_apply_once_and_keep_original_snapshot(monkeypatch) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    original_style = FakeWin32Con.WS_CAPTION | FakeWin32Con.WS_THICKFRAME | 0x10000000
    gui = FakeWin32Gui(original_style)
    manager = _manager(gui)
    assert manager.apply_automation_style()
    original_snapshot = manager._style_snapshot

    for resolution, position, expected in [
        ("1280x720", "top_right", (640, 0, 1920, 720)),
        ("1280x720", "bottom_left", (0, 320, 1280, 1040)),
        ("1920x1080", "unrestricted", (0, 320, 1920, 1400)),
    ]:
        manager.config = WindowConfig(resolution=resolution, position=position)
        previous_count = len(gui.calls)
        assert manager.apply_automation_style()
        assert len(gui.calls) == previous_count + 1
        assert gui.window_rect == expected
        assert manager._style_snapshot is original_snapshot
        assert manager.apply_automation_style()
        assert len(gui.calls) == previous_count + 1

    assert manager.restore_automation_style()
    assert gui.style == original_style
    assert gui.window_rect == (100, 200, 1400, 950)
    assert manager._applied_window_settings is None


def test_changed_settings_use_current_monitor(monkeypatch) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    gui = FakeWin32Gui(FakeWin32Con.WS_CAPTION)
    manager = _manager(gui)
    assert manager.apply_automation_style()
    gui.work_area = (-1920, -200, 0, 840)
    manager.config = WindowConfig(position="bottom_right")

    assert manager.apply_automation_style()

    assert gui.window_rect == (-1600, -60, 0, 840)


def test_failed_setting_change_retries_without_replacing_snapshot(monkeypatch) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    gui = FakeWin32Gui(FakeWin32Con.WS_CAPTION)
    manager = _manager(gui)
    assert manager.apply_automation_style()
    snapshot = manager._style_snapshot
    manager.config = WindowConfig(resolution="1280x720", position="center")
    set_window_pos = gui.SetWindowPos

    def fail_position_once(*args) -> None:
        gui.SetWindowPos = set_window_pos
        raise OSError("temporary resize failure")

    gui.SetWindowPos = fail_position_once
    with pytest.raises(OSError, match="temporary resize"):
        manager.apply_automation_style()
    assert manager._style_snapshot is snapshot
    assert manager._applied_window_settings == ("1600x900", "top_right")

    assert manager.apply_automation_style()
    assert gui.window_rect == (320, 160, 1600, 880)
    assert manager._applied_window_settings == ("1280x720", "center")


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


def test_engine_reload_applies_window_options_on_next_style_check(monkeypatch) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)

    class Writer:
        async def send(self, message: dict) -> None:
            pass

    gui = FakeWin32Gui(FakeWin32Con.WS_CAPTION)
    manager = _manager(gui)
    engine = AutomationEngine(EngineConfig())
    engine.writer = Writer()
    engine.context.window = manager
    engine.context.input = SimpleNamespace()
    window = WindowInfo(42, "Game", "Class", 1, Rect(0, 0, 1920, 1080), 1.0)

    async def reload_and_apply() -> None:
        await engine._ensure_window_style(window)
        snapshot = manager._style_snapshot
        await engine._reload_settings({
            "settings": {"game_window": {"resolution": "1280x720", "position": "bottom_left"}},
        })
        await engine._ensure_window_style(window)
        assert gui.window_rect == (0, 320, 1280, 1040)
        assert manager._style_snapshot is snapshot
        call_count = len(gui.calls)
        await engine._ensure_window_style(window)
        assert len(gui.calls) == call_count
        await engine._restore_window_style()
        assert gui.window_rect == (100, 200, 1400, 950)

    asyncio.run(reload_and_apply())


def test_partial_restore_keeps_original_snapshot_for_retry(monkeypatch) -> None:
    monkeypatch.setattr("window.windows.sleep", lambda _: None)
    original_style = FakeWin32Con.WS_CAPTION | FakeWin32Con.WS_THICKFRAME | 0x10000000
    gui = FakeWin32Gui(original_style)
    manager = _manager(gui)
    manager.apply_automation_style()
    original_snapshot = manager._style_snapshot
    set_window_pos = gui.SetWindowPos

    def fail_position_once(*args) -> None:
        gui.SetWindowPos = set_window_pos
        raise OSError("temporary frame refresh failure")

    gui.SetWindowPos = fail_position_once
    with pytest.raises(OSError, match="temporary frame refresh"):
        manager.restore_automation_style()
    assert manager._style_snapshot is original_snapshot

    assert manager.restore_automation_style()
    assert gui.style == original_style
    assert gui.window_rect == (100, 200, 1400, 950)
    assert manager._style_snapshot is None
