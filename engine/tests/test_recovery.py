from recovery.timeout import StateTimeout


def test_timeout_is_initially_not_expired() -> None:
    assert not StateTimeout(60).expired
