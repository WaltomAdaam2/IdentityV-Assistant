import pytest

from config.loader import parse_config


def test_config_accepts_default_shape() -> None:
    config = parse_config({"engine": {"dry_run": True}, "game_window": {"process_names": ["game.exe"], "title_contains": ["Game"]}})
    assert config.dry_run is True
    assert config.window.process_names == ("game.exe",)


def test_config_rejects_bad_threshold() -> None:
    with pytest.raises(ValueError, match="template_threshold"):
        parse_config({"vision": {"template_threshold": 2}})


def test_config_rejects_non_positive_capture_rate() -> None:
    with pytest.raises(ValueError, match="capture.idle_fps"):
        parse_config({"capture": {"idle_fps": 0}})
