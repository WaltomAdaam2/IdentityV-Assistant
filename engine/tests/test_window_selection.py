from config.schema import WindowConfig
from core.models import Rect
from window.base import WindowInfo
from window.windows import WindowsWindowManager


def test_window_match_reasons_are_case_insensitive() -> None:
    manager = WindowsWindowManager(WindowConfig(
        process_names=("dwrg.exe",),
        title_contains=("第五人格",),
        class_names=("GameWindow",),
    ))

    assert manager._match_reasons("第五人格 - 网易", "gamewindow", "DWRG.EXE") == ("title", "class", "process")


def test_window_rank_prefers_available_foreground_window() -> None:
    minimized = WindowInfo(1, "Game", "Class", 10, Rect(0, 0, 1920, 1080), 1.0, is_minimized=True)
    background = WindowInfo(2, "Game", "Class", 11, Rect(0, 0, 1920, 1080), 1.0)
    foreground = WindowInfo(3, "Game", "Class", 12, Rect(0, 0, 1280, 720), 1.0, is_foreground=True)

    ordered = sorted([minimized, background, foreground], key=WindowsWindowManager._window_rank)

    assert ordered == [foreground, background, minimized]


def test_configured_process_prevents_assistant_title_self_match() -> None:
    manager = WindowsWindowManager(WindowConfig(
        process_names=("dwrg.exe",),
        title_contains=("Identity V",),
    ))

    assistant_reasons = manager._match_reasons(
        "Identity V Assistant",
        "WebViewHost",
        "identityv-assistant.exe",
    )
    game_reasons = manager._match_reasons("Identity V", "GameWindow", "DWRG.EXE")

    assert assistant_reasons == ("title",)
    assert not manager._is_configured_match(assistant_reasons)
    assert manager._is_configured_match(game_reasons)


def test_title_matching_remains_available_without_process_filter() -> None:
    manager = WindowsWindowManager(WindowConfig(
        process_names=(),
        title_contains=("Capture Test",),
    ))

    assert manager._is_configured_match(("title",))
