from dataclasses import dataclass, field
from time import monotonic
from typing import Any

from states.enum import GameState


@dataclass(frozen=True)
class StateEvidence:
    state: GameState
    confidence: float
    source: str
    details: dict[str, Any] = field(default_factory=dict)
    observed_at: float = field(default_factory=monotonic)
