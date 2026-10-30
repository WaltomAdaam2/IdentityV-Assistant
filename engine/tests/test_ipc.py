import pytest

from ipc.messages import event
from ipc.protocol import ProtocolError, decode, encode, validate_envelope


def test_event_round_trips_as_single_ndjson_object() -> None:
    message = event("ready", engine_version="0.1.0")
    assert decode(encode(message)) == message


def test_command_validation_rejects_event_type() -> None:
    with pytest.raises(ProtocolError, match="unsupported command"):
        validate_envelope(event("ready"), commands_only=True)


@pytest.mark.parametrize("command", ["reload_settings", "enable_window_layout"])
def test_settings_and_layout_commands_are_valid(command: str) -> None:
    validate_envelope(event(command, settings={}), commands_only=True)
