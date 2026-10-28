from types import SimpleNamespace

import pytest

from window.automation import WindowStyleController


C = SimpleNamespace(
    GWL_STYLE=-16, GWL_EXSTYLE=-20,
    WS_CAPTION=0x00C00000, WS_THICKFRAME=0x00040000, WS_BORDER=0x00800000,
    WS_MAXIMIZEBOX=0x00010000, WS_MINIMIZE=0x20000000, WS_MAXIMIZE=0x01000000,
    WS_EX_CLIENTEDGE=0x00000200, WS_EX_WINDOWEDGE=0x00000100,
    WS_EX_DLGMODALFRAME=0x00000001, WS_EX_STATICEDGE=0x00020000, WS_EX_TOPMOST=0x00000008,
    SW_RESTORE=9, SWP_FRAMECHANGED=0x0020, SWP_NOMOVE=0x0002,
    SWP_NOSIZE=0x0001, SWP_NOZORDER=0x0004, SWP_NOACTIVATE=0x0010, SWP_SHOWWINDOW=0x0040,
    HWND_TOPMOST=-1, HWND_NOTOPMOST=-2, MONITOR_DEFAULTTONEAREST=2,
)


class FakeWin32:
    """Model window state and frame recomputation without invoking Windows."""

    def __init__(self, *, topmost=False, minimized=False):
        self.hwnd = 7
        self.style = C.WS_CAPTION | C.WS_THICKFRAME | C.WS_MAXIMIZEBOX | 0x00080000
        if minimized:
            self.style |= C.WS_MINIMIZE
        self.exstyle = C.WS_EX_CLIENTEDGE | C.WS_EX_WINDOWEDGE | 0x00040000
        if topmost:
            self.exstyle |= C.WS_EX_TOPMOST
        self.rect = (120, 140, 1420, 920)
        self.work = (-1920, 40, 0, 1080)
        self.frame = (12, 36)
        self.menu_height = 0
        self.mutations = []
        self.fail_next_position = False
        self.deny_foreground = False

    def IsWindow(self, hwnd):
        return hwnd == self.hwnd

    def GetWindowLong(self, hwnd, index):
        return self.style if index == C.GWL_STYLE else self.exstyle

    def SetWindowLong(self, hwnd, index, value):
        self.mutations.append(("style", index, value))
        if index == C.GWL_STYLE:
            self.style = value
        else:
            self.exstyle = value

    def GetWindowRect(self, hwnd):
        return self.rect

    def GetClientRect(self, hwnd):
        left, top, right, bottom = self.rect
        return (0, 0, right - left - self.frame[0], bottom - top - self.frame[1] - self.menu_height)

    def IsIconic(self, hwnd):
        return bool(self.style & C.WS_MINIMIZE)

    def ShowWindow(self, hwnd, command):
        assert command == C.SW_RESTORE
        self.mutations.append(("restore", hwnd))
        self.style &= ~(C.WS_MINIMIZE | C.WS_MAXIMIZE)
        return False  # ShowWindow reports the previous visibility, not success.

    def SetWindowPos(self, hwnd, insert_after, x, y, width, height, flags):
        self.mutations.append(("position", insert_after, x, y, width, height, flags))
        if self.fail_next_position:
            self.fail_next_position = False
            raise OSError("Window refused the change")
        left, top, right, bottom = self.rect
        if flags & C.SWP_NOSIZE:
            width, height = right - left, bottom - top
        if flags & C.SWP_NOMOVE:
            x, y = left, top
        self.rect = (x, y, x + width, y + height)
        if not flags & C.SWP_NOZORDER:
            if insert_after == C.HWND_TOPMOST:
                self.exstyle |= C.WS_EX_TOPMOST
            elif insert_after == C.HWND_NOTOPMOST:
                self.exstyle &= ~C.WS_EX_TOPMOST
        if flags & C.SWP_FRAMECHANGED:
            self.frame = (12, 36) if self.style & (C.WS_CAPTION | C.WS_THICKFRAME) else (0, 0)

    def MonitorFromWindow(self, hwnd, flags):
        assert flags == C.MONITOR_DEFAULTTONEAREST
        return "nearest"

    def GetMonitorInfo(self, monitor):
        assert monitor == "nearest"
        return {"Work": self.work}

    def SetForegroundWindow(self, hwnd):
        self.mutations.append(("foreground", hwnd))
        if self.deny_foreground:
            raise OSError("Foreground permission denied")


def controller(native):
    return WindowStyleController(gui=native, api=native, constants=C)


def test_requested_resolution_is_client_size_even_with_remaining_nonclient_area():
    native = FakeWin32(minimized=True)
    native.menu_height = 20
    layout = controller(native)

    assert layout.apply(7, "1600x900", "top_right")
    assert native.GetClientRect(7) == (0, 0, 1600, 900)
    assert native.rect == (-1600, 40, 0, 960)
    assert not native.style & (C.WS_CAPTION | C.WS_THICKFRAME | C.WS_BORDER | C.WS_MAXIMIZEBOX)
    assert not native.IsIconic(7)
    assert native.exstyle & C.WS_EX_TOPMOST
    assert not native.exstyle & (C.WS_EX_CLIENTEDGE | C.WS_EX_WINDOWEDGE)


@pytest.mark.parametrize(
    ("position", "expected"),
    [
        ("top_left", (-1920, 40)),
        ("top_right", (-1600, 40)),
        ("bottom_left", (-1920, 180)),
        ("bottom_right", (-1600, 180)),
        ("center", (-1760, 110)),
    ],
)
def test_positions_use_nearest_monitor_work_area_with_negative_coordinates(position, expected):
    native = FakeWin32()
    assert controller(native).apply(7, "1600x900", position)
    assert native.rect[:2] == expected


@pytest.mark.parametrize("position", ["center", "top_right", "bottom_right"])
def test_oversized_window_keeps_requested_resolution_and_visible_top_left(position):
    native = FakeWin32()
    native.work = (100, 80, 1100, 680)
    assert controller(native).apply(7, "1600x900", position)
    assert native.rect == (100, 80, 1700, 980)
    assert native.GetClientRect(7) == (0, 0, 1600, 900)


@pytest.mark.parametrize("work", [(0, 0, 1920, 1080), (-1920, 40, 0, 1080), (100, 80, 1100, 680)])
def test_unrestricted_preserves_exact_current_placement(work):
    native = FakeWin32()
    native.work = work
    assert controller(native).apply(7, "1600x900", "unrestricted")
    assert native.rect[:2] == (120, 140)


def test_repeated_same_configuration_has_no_window_mutations():
    native = FakeWin32()
    layout = controller(native)
    assert layout.apply(7, "1600x900", "center")
    mutations = list(native.mutations)
    assert layout.apply(7, "1600x900", "center")
    assert native.mutations == mutations


@pytest.mark.parametrize("topmost", [False, True])
def test_config_changes_retain_original_state_for_restore(topmost):
    native = FakeWin32(topmost=topmost)
    original = native.style, native.exstyle, native.rect
    layout = controller(native)
    assert layout.apply(7, "1600x900", "center")
    assert layout.apply(7, "1280x720", "bottom_left")
    assert layout.restore()
    assert (native.style, native.exstyle, native.rect) == original
    mutations = list(native.mutations)
    assert layout.restore()
    assert native.mutations == mutations


def test_failed_restore_preserves_snapshot_for_retry():
    native = FakeWin32()
    original = native.style, native.exstyle, native.rect
    layout = controller(native)
    assert layout.apply(7, "1600x900", "center")
    native.fail_next_position = True
    assert not layout.restore()
    assert layout.restore()
    assert (native.style, native.exstyle, native.rect) == original


@pytest.mark.parametrize("restore_before_retarget", [False, True])
def test_closed_window_does_not_block_retargeting(restore_before_retarget):
    native = FakeWin32()
    layout = controller(native)
    assert layout.apply(7, "1600x900", "center")
    native.hwnd = 8
    native.rect = (200, 160, 1500, 940)
    replacement_original = native.style, native.exstyle, native.rect
    if restore_before_retarget:
        mutations = list(native.mutations)
        assert layout.restore()
        assert native.mutations == mutations

    assert layout.apply(8, "1280x720", "top_left")
    assert native.GetClientRect(8) == (0, 0, 1280, 720)
    assert layout.restore()
    assert (native.style, native.exstyle, native.rect) == replacement_original


def test_failed_apply_can_restore_partially_changed_window():
    native = FakeWin32()
    original = native.style, native.exstyle, native.rect
    layout = controller(native)
    native.fail_next_position = True
    assert not layout.apply(7, "1600x900", "center")
    assert layout.restore()
    assert (native.style, native.exstyle, native.rect) == original


def test_foreground_denial_does_not_fail_successful_layout():
    native = FakeWin32()
    native.deny_foreground = True
    assert controller(native).apply(7, "1600x900", "center")
    assert native.GetClientRect(7) == (0, 0, 1600, 900)


@pytest.mark.parametrize(
    ("hwnd", "resolution", "position"),
    [(8, "1600x900", "center"), (7, "0x900", "center"), (7, "invalid", "center"), (7, "1600x900", "invalid")],
)
def test_invalid_request_leaves_window_untouched(hwnd, resolution, position):
    native = FakeWin32()
    assert not controller(native).apply(hwnd, resolution, position)
    assert native.mutations == []
