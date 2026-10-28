"""NDJSON entry point. stdout is reserved exclusively for protocol messages."""
from __future__ import annotations

import argparse
import asyncio
import sys
from pathlib import Path

from app.engine import AutomationEngine
from config.loader import load_config
from core.resources import resource_root


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Identity V Assistant automation engine")
    parser.add_argument(
        "--config",
        type=Path,
        default=resource_root() / "config" / "default.json",
    )
    parser.add_argument("--self-test", action="store_true", help="Check packaged dependencies without touching the game")
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
    engine = AutomationEngine(config)
    await engine.run()
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(run()))
