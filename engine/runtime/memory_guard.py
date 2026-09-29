from __future__ import annotations

import gc
from collections.abc import Callable


class MemoryGuard:
    """Release cached objects when total system memory use is critically high."""

    def __init__(self, enabled: bool = False, threshold_percent: float = 90.0) -> None:
        self.enabled = enabled
        self.threshold_percent = threshold_percent
        self._cleaners: list[Callable[[], None]] = []

    def register_cleaner(self, cleaner: Callable[[], None]) -> None:
        self._cleaners.append(cleaner)

    def protect(self) -> float | None:
        if not self.enabled:
            return None

        import psutil

        percent = float(psutil.virtual_memory().percent)
        if percent <= self.threshold_percent:
            return percent
        for cleaner in self._cleaners:
            cleaner()
        gc.collect()
        return percent
