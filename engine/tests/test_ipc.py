import pytest

from ipc.messages import event
from ipc.protocol import ProtocolError, decode, encode, validate_envelope


def test_event_round_trips_as_single_ndjson_object() -> None:
    message = event("ready", engine_version="0.1.0")
    assert decode(encode(message)) == message


def test_command_validation_rejects_event_type() -> None:
    with pytest.raises(ProtocolError, match="unsupported command"):
        validate_envelope(event("ready"), commands_only=True)


def test_reload_settings_is_a_valid_command() -> None:
    validate_envelope(event("reload_settings", settings={}), commands_only=True)
