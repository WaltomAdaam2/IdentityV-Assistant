import sys
from types import SimpleNamespace

from runtime.memory_guard import MemoryGuard


def test_memory_guard_cleans_only_above_ninety_percent(monkeypatch) -> None:
    cleaner_calls: list[str] = []
    guard = MemoryGuard(enabled=True)
    guard.register_cleaner(lambda: cleaner_calls.append("clean"))
    monkeypatch.setitem(
        sys.modules,
        "psutil",
        SimpleNamespace(virtual_memory=lambda: SimpleNamespace(percent=91.0)),
    )
    monkeypatch.setattr("runtime.memory_guard.gc.collect", lambda: cleaner_calls.append("gc"))

    assert guard.protect() == 91.0
    assert cleaner_calls == ["clean", "gc"]


def test_memory_guard_is_inactive_by_default() -> None:
    assert MemoryGuard().protect() is None
