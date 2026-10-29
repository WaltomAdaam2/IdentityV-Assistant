"""Local lifecycle fixture. Does not implement game login or modify the system."""
import argparse
import json
import os
from pathlib import Path
import signal
import ctypes
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlsplit

parser = argparse.ArgumentParser()
parser.add_argument("--proxy-port", type=int, required=True)
args = parser.parse_args()
root = Path(os.environ["PROGRAMDATA"])
mode_file = root / "fixture-mode"
mode = mode_file.read_text(encoding="utf-8") if mode_file.exists() else "process"
if mode == "exit":
    raise SystemExit(3)


def stop(*_args):
    (root / "stopped").write_text("graceful", encoding="utf-8")
    os._exit(0)


signal.signal(signal.SIGBREAK, stop)
signal.signal(signal.SIGINT, stop)
console_handler = ctypes.WINFUNCTYPE(ctypes.c_int, ctypes.c_uint)(lambda event: stop(event) or 1)
ctypes.windll.kernel32.SetConsoleCtrlHandler(console_handler, True)


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        path = urlsplit(self.path).path
        if path == "/_idv-login/health":
            body = {"success": True, "status": "starting" if mode == "not-ready" else "ok", "version": "v6.3.2-stable"}
        elif path == "/_idv-login/proxy-mode":
            body = {"success": True, "mode": mode}
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
