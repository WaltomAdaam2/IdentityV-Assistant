from __future__ import annotations

import json
from typing import Any

from ipc.messages import COMMAND_TYPES, PROTOCOL_VERSION


class ProtocolError(ValueError):
    pass


def encode(message: dict[str, Any]) -> str:
    validate_envelope(message)
    return json.dumps(message, ensure_ascii=False, separators=(",", ":"))


def decode(line: str) -> dict[str, Any]:
    try:
        message = json.loads(line)
    except json.JSONDecodeError as error:
        raise ProtocolError(f"invalid JSON: {error.msg}") from error
    validate_envelope(message)
    return message


def validate_envelope(message: Any, commands_only: bool = False) -> None:
    if not isinstance(message, dict):
        raise ProtocolError("message must be an object")
    if message.get("version") != PROTOCOL_VERSION:
        raise ProtocolError(f"unsupported protocol version: {message.get('version')!r}")
    if not isinstance(message.get("type"), str) or not message["type"]:
        raise ProtocolError("message type is required")
    if not isinstance(message.get("timestamp"), str) or not message["timestamp"]:
        raise ProtocolError("message timestamp is required")
    if commands_only and message["type"] not in COMMAND_TYPES:
        raise ProtocolError(f"unsupported command: {message['type']}")
