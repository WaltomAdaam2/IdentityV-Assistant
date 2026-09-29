from dataclasses import asdict, dataclass, field
from time import monotonic


@dataclass
class RuntimeMetrics:
    screenshots: int = 0
    state_changes: int = 0
    recovery_attempts: int = 0
    recognition_failures: int = 0
    _started: float = field(default_factory=monotonic)

    def snapshot(self) -> dict[str, int | float]:
        data = asdict(self)
        data.pop("_started")
        data["runtime_seconds"] = round(monotonic() - self._started, 3)
        return data
