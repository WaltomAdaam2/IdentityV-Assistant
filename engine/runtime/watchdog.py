from time import monotonic


class Watchdog:
    def __init__(self, timeout_seconds: float = 10.0) -> None:
        self.timeout_seconds = timeout_seconds
        self._last_feed = monotonic()

    def feed(self) -> None:
        self._last_feed = monotonic()

    @property
    def expired(self) -> bool:
        return monotonic() - self._last_feed > self.timeout_seconds
