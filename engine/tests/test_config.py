import pytest

from config.loader import parse_config
from config.schema import EngineConfig


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


def test_config_models_are_frozen_and_validate_direct_construction() -> None:
    config = EngineConfig(capture={"idle_fps": 5})
    assert config.capture.idle_fps == 5
    with pytest.raises(ValueError):
        EngineConfig(capture={"idle_fps": True})
