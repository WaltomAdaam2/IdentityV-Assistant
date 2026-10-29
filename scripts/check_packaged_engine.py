"""Verify an installed-folder engine without Python on PATH or any game input."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
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
        marker = "不存在的打包测试窗口-" + uuid.uuid4().hex
        config["game_window"].update(process_names=[marker + ".exe"], title_contains=[marker], class_names=[marker])
        config_file = root / "config.json"
        config_file.write_text(json.dumps(config), encoding="utf-8")
        commands = "".join(json.dumps({"version": 1, "type": command, "request_id": "中文请求-" + command,
                                      "timestamp": "2026-01-01T00:00:00Z"}, ensure_ascii=False) + "\n"
                           for command in ("detect_window", "capture_frame", "shutdown"))
        with subprocess.Popen([str(executable), "--config", str(config_file)], cwd=root, env=env,
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              text=True, encoding="utf-8", creationflags=subprocess.CREATE_NO_WINDOW) as process:
            try:
                # Exercise idle heartbeats before non-ASCII request/response traffic.
                time.sleep(6)
                stdout, stderr = process.communicate(commands, timeout=30)
            except BaseException:
                process.kill()
                process.wait()
                raise
            if process.returncode:
                raise RuntimeError(stdout + stderr)
        events = [json.loads(line) for line in stdout.splitlines() if line.strip()]
        if not any(event.get("type") == "ready" for event in events):
            raise RuntimeError("Frozen engine did not send ready: " + stdout + stderr)
        if sum(event.get("type") == "heartbeat" for event in events) < 2:
            raise RuntimeError("Frozen engine did not sustain idle heartbeats")
        if not any(event.get("type") == "window_diagnostics" and event.get("request_id") == "中文请求-detect_window" for event in events):
            raise RuntimeError("Frozen engine did not preserve a Unicode diagnostic request")
        if not any(event.get("code") == "WINDOW_NOT_FOUND" and marker in event.get("details", {}).get("title_contains", []) for event in events):
            raise RuntimeError("Frozen engine did not preserve Unicode window details")
        if not any(event.get("code") == "WINDOW_NOT_FOUND" and event.get("request_id") == "中文请求-capture_frame" for event in events):
            raise RuntimeError("Frozen engine did not handle capture after window detection")
        print("Installed engine: Unicode path/IPC, offline models, no Python PATH, hidden-process heartbeats/detect/capture/shutdown passed.")


if __name__ == "__main__":
    check(Path(sys.argv[1]).resolve())
