from time import monotonic


class StateTimeout:
    def __init__(self, seconds: float) -> None:
        self.seconds = seconds
        self._started = monotonic()

    @property
    def expired(self) -> bool:
        return monotonic() - self._started >= self.seconds
