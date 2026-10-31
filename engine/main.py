"""NDJSON entry point. stdout is reserved exclusively for protocol messages."""
from __future__ import annotations

import argparse
import asyncio
import os
import sys
from pathlib import Path

# OpenCV reads this before its first import; bound decoded map allocation too.
os.environ.setdefault("OPENCV_IO_MAX_IMAGE_PIXELS", "16777216")
from config.loader import load_config
from core.resources import resource_root


def configure_stdio() -> None:
    # Frozen Python ignores PYTHONIOENCODING; the Rust IPC reader requires UTF-8.
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8", newline="\n", write_through=True)
    sys.stderr.reconfigure(encoding="utf-8", errors="backslashreplace", newline="\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Identity V Assistant automation engine")
    parser.add_argument(
        "--config",
        type=Path,
        default=resource_root() / "config" / "default.json",
    )
    parser.add_argument("--self-test", action="store_true", help="Check packaged dependencies without touching the game")
    parser.add_argument("--recognize-journal", action="store_true", help="Capture and recognize once without starting automation")
    return parser.parse_args()


async def run() -> int:
    args = parse_args()
    if args.self_test:
        from core.package_check import check_package
        check_package()
        return 0
    try:
        config = load_config(args.config)
    except ValueError as error:
        # stdout is unavailable until the writer exists; stderr preserves IPC integrity.
        print(f"CONFIG_INVALID: {error}", file=sys.stderr)
        return 2
    if args.recognize_journal:
        from app.journal_recognition import main
        return main(config)
    from app.engine import AutomationEngine
    engine = AutomationEngine(config)
    await engine.run()
    return 0


if __name__ == "__main__":
    configure_stdio()
    raise SystemExit(asyncio.run(run()))
