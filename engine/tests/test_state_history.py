from states.evidence import StateEvidence
from states.enum import GameState
from states.history import StateHistory


def test_state_requires_configured_confirmation_count() -> None:
    history = StateHistory(confirmation_count=2)
    assert history.observe(StateEvidence(GameState.LOBBY, 0.9, "test")) is None
    transition = history.observe(StateEvidence(GameState.LOBBY, 0.9, "test"))
    assert transition is not None
    assert transition.previous == GameState.UNKNOWN
    assert transition.current == GameState.LOBBY


def test_unknown_frames_are_tracked() -> None:
    history = StateHistory(1)
    history.observe(StateEvidence(GameState.UNKNOWN, 0.0, "test"))
    assert history.unknown_frames == 1
