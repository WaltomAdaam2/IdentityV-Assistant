import asyncio
import io
import json
import sys

from ipc.writer import NdjsonWriter
from main import configure_stdio


def test_frozen_locale_streams_use_utf8_for_protocol_and_errors(monkeypatch):
    incoming = io.BytesIO("检测窗口\n".encode("utf-8"))
    outgoing = io.BytesIO()
    errors = io.BytesIO()
    streams = [io.TextIOWrapper(buffer, encoding="gbk") for buffer in (incoming, outgoing, errors)]
    with monkeypatch.context() as patch:
        for name, stream in zip(("stdin", "stdout", "stderr"), streams):
            patch.setattr(sys, name, stream)
        configure_stdio()
        received = sys.stdin.readline()
        asyncio.run(NdjsonWriter().send({"version": 1, "type": "log", "timestamp": "test", "message": "检测窗口 🗺"}))
        print("诊断信息 🗺", file=sys.stderr, flush=True)
    assert received == "检测窗口\n"
    assert json.loads(outgoing.getvalue().decode("utf-8"))["message"] == "检测窗口 🗺"
    assert errors.getvalue().decode("utf-8") == "诊断信息 🗺\n"
