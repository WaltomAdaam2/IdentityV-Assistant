from types import SimpleNamespace

import pytest

from config.schema import WindowConfig
from core.models import Rect
from window.windows import WindowsWindowManager


def window_state(pid=100):
    return SimpleNamespace(
        pid=pid, title="Game", class_name="GameClass", visible=True, alive=True,
        minimized=False, responding=True, rect=Rect(100, 200, 1600, 900), dpi=96,
    )


class FakeUser32:
    def __init__(self):
        self.windows = {42: window_state()}
        self.foreground = 42
        self.enumerations = 0
        self.probes = []

    def EnumWindows(self, callback, parameter):
        self.enumerations += 1
        for hwnd in self.windows:
            callback(hwnd, parameter)
        return True

    def IsWindow(self, hwnd):
        return hwnd in self.windows and self.windows[hwnd].alive

    def IsWindowVisible(self, hwnd):
        return self.IsWindow(hwnd) and self.windows[hwnd].visible

    def GetWindowTextLengthW(self, hwnd):
        return len(self.windows[hwnd].title)

    def GetWindowTextW(self, hwnd, buffer, length):
        buffer.value = self.windows[hwnd].title

    def GetClassNameW(self, hwnd, buffer, length):
        buffer.value = self.windows[hwnd].class_name

    def GetWindowThreadProcessId(self, hwnd, pid):
        pid._obj.value = self.windows[hwnd].pid
        return 1

    def GetClientRect(self, hwnd, rect):
        source = self.windows[hwnd].rect
        rect._obj.left, rect._obj.top = 0, 0
        rect._obj.right, rect._obj.bottom = source.width, source.height
        return True

    def ClientToScreen(self, hwnd, point):
        source = self.windows[hwnd].rect
        point._obj.x, point._obj.y = source.left, source.top
        return True

    def IsIconic(self, hwnd):
        return self.windows[hwnd].minimized

    def GetForegroundWindow(self):
        return self.foreground

    def GetDpiForWindow(self, hwnd):
        return self.windows[hwnd].dpi

    def SendMessageTimeoutW(self, hwnd, *args):
        self.probes.append(hwnd)
        assert args[-2] == 250
        return self.windows[hwnd].responding


@pytest.fixture
def cached_manager(monkeypatch):
    now = [0.0]
    monkeypatch.setattr("window.windows.sys.platform", "win32")
    monkeypatch.setattr("window.windows.monotonic", lambda: now[0])
    monkeypatch.setattr(WindowsWindowManager, "_configure_winapi", lambda _: None)
    manager = WindowsWindowManager(WindowConfig(process_names=["Game.exe"]))
    user32 = FakeUser32()
    manager._user32 = user32
    process_lookups = []

    def process_name(pid):
        process_lookups.append(pid)
        return "Other.exe" if pid == 200 else "Game.exe"

    manager._process_name = process_name
    return manager, user32, now, process_lookups


def test_frequent_window_reads_keep_geometry_and_focus_current_without_repeated_discovery(cached_manager):
    manager, user32, now, process_lookups = cached_manager
    assert manager.find_window().hwnd == 42
    for step in range(1, 50):
        now[0] = step / 100
        assert manager.find_window().hwnd == 42
        assert manager.is_responding()
    assert user32.enumerations == len(process_lookups) == len(user32.probes) == 1

    state = user32.windows[42]
    state.rect = Rect(-1600, 0, 1280, 720)
    state.minimized = True
    state.dpi = 144
    user32.foreground = 99
    current = manager.find_window()
    assert current.client_rect == state.rect
    assert current.is_minimized and not current.is_foreground
    assert current.dpi_scale == 1.5
    assert user32.enumerations == len(user32.probes) == 1

    now[0] = 1.01
    state.responding = False
    assert not manager.find_window().is_responding
    assert user32.enumerations == len(process_lookups) == len(user32.probes) == 2


@pytest.mark.parametrize("invalid", ["closed", "hidden", "reused"])
def test_invalid_cached_handle_immediately_rediscovers_the_game(cached_manager, invalid):
    manager, user32, now, _ = cached_manager
    assert manager.find_window().hwnd == 42
    if invalid == "closed":
        user32.windows[42].alive = False
    elif invalid == "hidden":
        user32.windows[42].visible = False
    else:
        user32.windows[42].pid = 200
    user32.windows[77] = window_state(pid=300)
    user32.foreground = 77
    now[0] = 0.1
    assert manager.find_window().hwnd == 77
    assert user32.enumerations == 2
    assert user32.probes == [42, 77]


def test_title_matching_refreshes_at_expiry_and_config_changes_refresh_immediately(cached_manager):
    manager, user32, now, _ = cached_manager
    manager.config = WindowConfig(process_names=[], class_names=[], title_contains=["Game"])
    assert manager.find_window().hwnd == 42
    user32.windows[42].title = "Changed"
    now[0] = 1.01
    assert manager.find_window() is None
    assert user32.enumerations == 2

    manager.config = manager.config.model_copy(update={"title_contains": ["Changed"]})
    now[0] = 1.02
    assert manager.find_window().title == "Changed"
    assert user32.enumerations == 3


def test_missing_window_scan_is_bounded_and_recovers_on_next_refresh(cached_manager):
    manager, user32, now, _ = cached_manager
    user32.windows[42].alive = False
    assert manager.find_window() is None
    user32.windows[77] = window_state(pid=300)
    now[0] = 0.5
    assert manager.find_window() is None
    assert user32.enumerations == 1
    now[0] = 1.01
    assert manager.find_window().hwnd == 77
    assert user32.enumerations == 2
