"""Local lifecycle fixture. Does not implement game login or modify the system."""
import argparse
import json
import os
from pathlib import Path
import signal
import ctypes
import threading
import sys
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlsplit

parser = argparse.ArgumentParser()
parser.add_argument("--proxy-port", type=int, required=True)
args = parser.parse_args()
root = Path(os.environ["PROGRAMDATA"])
mode_file = root / "fixture-mode"
mode = mode_file.read_text(encoding="utf-8") if mode_file.exists() else "process"
(root / "console-window").write_text(str(ctypes.windll.kernel32.GetConsoleWindow()), encoding="ascii")
if mode == "exit":
    raise SystemExit(3)
if mode == "normal-exit":
    raise SystemExit(0)
if mode == "restart-always":
    raise SystemExit(75)
if mode == "update-prompt":
    print("IDV_ASSISTANT_STAGE updating", flush=True)
if mode == "restart-once" and not (root / "restarted").exists():
    (root / "restarted").write_text("requested", encoding="ascii")
    raise SystemExit(75)


def stop(*_args):
    (root / "stopped").write_text("graceful", encoding="utf-8")
    os._exit(0)


signal.signal(signal.SIGBREAK, stop)
signal.signal(signal.SIGINT, stop)
console_handler = ctypes.WINFUNCTYPE(ctypes.c_int, ctypes.c_uint)(lambda event: stop(event) or 1)
ctypes.windll.kernel32.SetConsoleCtrlHandler(console_handler, True)
threading.Thread(target=lambda: (sys.stdin.readline(), stop()), daemon=True).start()

restart_request = root / "restart-request"
initial_request = restart_request.read_text() if restart_request.exists() else ""
def watch_restart():
    while True:
        if restart_request.exists() and restart_request.read_text() != initial_request:
            os._exit(75)
        time.sleep(0.05)
threading.Thread(target=watch_restart, daemon=True).start()


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        path = urlsplit(self.path).path
        if path == "/_idv-login/health":
            unhealthy = root / "unhealthy"
            body = {"success": True, "status": "starting" if mode == "not-ready" or (unhealthy.exists() and unhealthy.read_text()) else "ok", "version": "v6.3.2-stable"}
        elif path == "/_idv-login/proxy-mode":
            body = {"success": True, "mode": "process" if mode in {"restart-once", "update-prompt"} else mode}
        else:
            self.send_error(404)
            return
        payload = json.dumps(body).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *_args):
        pass


HTTPServer(("127.0.0.1", args.proxy_port), Handler).serve_forever()
