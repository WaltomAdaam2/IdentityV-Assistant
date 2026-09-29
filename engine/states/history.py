from dataclasses import dataclass
from time import monotonic

from states.evidence import StateEvidence
from states.enum import GameState


@dataclass(frozen=True)
class StateTransition:
    previous: GameState
    current: GameState
    confidence: float
    duration_in_previous_state: float


class StateHistory:
    def __init__(self, confirmation_count: int = 2) -> None:
        self.confirmation_count = confirmation_count
        self.current = GameState.UNKNOWN
        self._candidate = GameState.UNKNOWN
        self._candidate_count = 0
        self._entered_at = monotonic()
        self.unknown_frames = 0

    def observe(self, evidence: StateEvidence) -> StateTransition | None:
        if evidence.state == GameState.UNKNOWN:
            self.unknown_frames += 1
        else:
            self.unknown_frames = 0
        if evidence.state == self.current:
            self._candidate_count = 0
            return None
        if evidence.state == self._candidate:
            self._candidate_count += 1
        else:
            self._candidate, self._candidate_count = evidence.state, 1
        if self._candidate_count < self.confirmation_count:
            return None
        now = monotonic()
        transition = StateTransition(self.current, evidence.state, evidence.confidence, now - self._entered_at)
        self.current, self._entered_at, self._candidate_count = evidence.state, now, 0
        return transition
