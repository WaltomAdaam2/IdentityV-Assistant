"""NDJSON entry point. stdout is reserved exclusively for protocol messages."""
from __future__ import annotations

import argparse
import asyncio
import sys
from pathlib import Path

from app.engine import AutomationEngine
from config.loader import load_config


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Identity V Assistant automation engine")
    parser.add_argument(
        "--config",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "config" / "default.json",
    )
    return parser.parse_args()


async def run() -> int:
    args = parse_args()
    try:
        config = load_config(args.config)
    except ValueError as error:
        # stdout is unavailable until the writer exists; stderr preserves IPC integrity.
        print(f"CONFIG_INVALID: {error}", file=sys.stderr)
        return 2
    engine = AutomationEngine(config)
    await engine.run()
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(run()))
