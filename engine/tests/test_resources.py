import sys
from pathlib import Path

from core.resources import read_template, resource_root


def test_checkout_resources():
    assert (resource_root() / "config" / "default.json").is_file()
    assert (resource_root() / "assets" / "templates" / "blackjack" / "hunter_icon.png").is_file()


def test_frozen_resources_use_bundle_not_working_directory(monkeypatch, tmp_path):
    monkeypatch.setattr(sys, "frozen", True, raising=False)
    monkeypatch.setattr(sys, "_MEIPASS", str(tmp_path), raising=False)
    assert resource_root() == Path(tmp_path)


def test_templates_load_from_chinese_install_directory(tmp_path):
    target = tmp_path / "第五人格 助手" / "hunter.png"
    target.parent.mkdir()
    target.write_bytes((resource_root() / "assets/templates/blackjack/hunter_icon.png").read_bytes())
    image = read_template(target)
    assert image is not None and image.size > 0
