from __future__ import annotations

import asyncio
import sys
from collections.abc import AsyncIterator

from ipc.protocol import ProtocolError, decode, validate_envelope


class NdjsonReader:
    async def messages(self) -> AsyncIterator[dict]:
        while True:
            line = await asyncio.to_thread(sys.stdin.readline)
            if not line:
                return
            try:
                message = decode(line)
                validate_envelope(message, commands_only=True)
            except ProtocolError:
                continue
            yield message
