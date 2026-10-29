"""Verify an installed-folder engine without Python on PATH or any game input."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import uuid


def check(source: Path) -> None:
    internal = source / "_internal"
    if not (internal / "THIRD_PARTY_LICENSES.txt").is_file():
        raise RuntimeError("Packaged third-party license notices are missing")
    for name in ("geos-3.13.1.tar.bz2", "shapely-2.1.2.tar.gz", "SOURCES.txt"):
        if not (internal / "third-party-sources" / name).is_file():
            raise RuntimeError(f"Packaged LGPL corresponding source is missing: {name}")
    if any("opencv_videoio_ffmpeg" in file.name.lower() for file in internal.rglob("*.dll")):
        raise RuntimeError("Unused FFmpeg plugin must not be bundled")
    with tempfile.TemporaryDirectory(prefix="IdentityV 安装检查 ") as directory:
        root = Path(directory)
        install = root / "安装目录" / "engine"
        shutil.copytree(source, install)
        executable = install / "identityv-engine.exe"
        env = dict(os.environ, PATH="", LOCALAPPDATA=str(root / "data"), PYTHONIOENCODING="utf-8")
        env.pop("PYTHONHOME", None)
        env.pop("PYTHONPATH", None)
        result = subprocess.run([str(executable), "--self-test"], cwd=root, env=env,
                                capture_output=True, text=True, encoding="utf-8", timeout=90)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        print(result.stdout.strip())
        config = json.loads((install / "_internal/config/default.json").read_text(encoding="utf-8"))
        # Never match a real game window during the IPC smoke test.
        marker = "package-test-" + uuid.uuid4().hex
        config["game_window"].update(process_names=[marker + ".exe"], title_contains=[marker], class_names=[marker])
        config_file = root / "config.json"
        config_file.write_text(json.dumps(config), encoding="utf-8")
        shutdown = json.dumps({"version": 1, "type": "shutdown", "timestamp": "2026-01-01T00:00:00Z"}) + "\n"
        result = subprocess.run([str(executable), "--config", str(config_file)], input=shutdown,
                                cwd=root, env=env, capture_output=True, text=True, encoding="utf-8", timeout=30)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        events = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
        if not any(event.get("type") == "ready" for event in events):
            raise RuntimeError("Frozen engine did not send ready: " + result.stdout + result.stderr)
        print("Installed engine: Unicode path, offline models, no Python PATH, NDJSON ready/shutdown passed.")


if __name__ == "__main__":
    check(Path(sys.argv[1]).resolve())
