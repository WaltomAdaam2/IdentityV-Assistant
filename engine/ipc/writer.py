from __future__ import annotations

import asyncio
import sys

from ipc.protocol import encode


class NdjsonWriter:
    """The sole owner of stdout, preserving valid one-line protocol output."""

    def __init__(self) -> None:
        self._lock = asyncio.Lock()

    async def send(self, message: dict) -> None:
        line = encode(message)
        async with self._lock:
            sys.stdout.write(line + "\n")
            sys.stdout.flush()
