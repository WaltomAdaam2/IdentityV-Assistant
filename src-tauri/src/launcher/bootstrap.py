"""Host lifecycle adapter for the pinned official runtime; no login protocol code."""
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import threading
import time

OUTPUT = sys.stdout
OUTPUT_LOCK = threading.Lock()


def stage(phase):
    with OUTPUT_LOCK:
        OUTPUT.write("IDV_ASSISTANT_STAGE " + phase + "\n")
        OUTPUT.flush()


def emit(level, message):
    if level not in {"INFO", "WARNING", "ERROR", "CRITICAL"}:
        return
    message = re.sub(r"(?im)(authorization|cookie)\s*[:=].*$", r"\1: [redacted]", str(message))
    message = re.sub(r"https?://[^\s\"']+", lambda m: m[0].split("?", 1)[0] + ("?[redacted]" if "?" in m[0] else ""), message)
    message = re.sub(r'''(?i)((?:access_?token|refresh_?token|token|password|passwd|authorization|cookie|secret|ticket|session_?id)\s*["']?\s*[:=]\s*)(?:"[^"]*"|'[^']*'|[^\s,;}]+)''', r"\1[redacted]", message)
    with OUTPUT_LOCK:
        OUTPUT.write("IDV_ASSISTANT_LOG " + json.dumps({"level": level, "message": message[:4000]}, ensure_ascii=False) + "\n")
        OUTPUT.flush()


def import_certificate(_self, cert_path):
    result = subprocess.run(
        [str(Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32/certutil.exe"), "-addstore", "Root", cert_path],
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        creationflags=0x08000000, timeout=30,
    )
    if result.returncode:
        raise RuntimeError(f"登录证书导入失败（{result.returncode}），请查看权限与证书状态")
    return True


def adapt(upstream):
    def skip(*_args, **_kwargs):
        emit("INFO", "由助手托管：跳过桌面快捷方式创建")

    def managed_restart(reason=""):
        stage("restarting")
        emit("INFO", f"组件请求重启，由助手重新建立代理：{reason}")
        upstream.genv.set("last_run_state", "restart", True)
        upstream.genv.set("last_run_state_ts", int(time.time()), True)
        try:
            upstream.handle_exit()
        finally:
            sys.stdout.flush()
            sys.stderr.flush()
            # This can run in an upstream Timer thread; SystemExit would leave
            # the proxy alive. The owner recognizes this process exit code.
            os._exit(75)

    def fail(error):
        upstream.genv.set("last_run_state", "crash", True)
        emit("CRITICAL", f"登录组件异常：{error}")
        raise SystemExit(1)

    upstream.setup_shortcuts = skip
    upstream.handle_error_and_exit = fail
    upstream.hotfixmgr.restart_self = managed_restart
    def managed_updates(*_args, **_kwargs):
        # The assistant checks at game launch and applies only the snapshot the
        # user confirmed. Do not open a second Qt prompt or start a countdown.
        return None

    upstream.handle_update = managed_updates
    upstream.hotfixmgr.handle_if_needed = managed_updates


def main():
    source = os.environ.get("IDV_ASSISTANT_SRC")
    if not source:
        python = Path(json.loads((Path(__file__).parent.parent / "launcher.json").read_text(encoding="utf-8")))
        source = str(python.parent.parent / "src")
    data = Path(os.environ["PROGRAMDATA"]) / "idv-login"
    data.mkdir(parents=True, exist_ok=True)
    os.chdir(data)  # Upstream logutil creates log.txt on import.
    sys.path.insert(0, source)
    import buildinfo
    if buildinfo.VERSION != "v6.3.2-stable":
        raise RuntimeError("不支持此登录组件版本")
    spec = importlib.util.spec_from_file_location("main", Path(source) / "main.pyc")
    upstream = importlib.util.module_from_spec(spec)
    sys.modules["main"] = upstream
    spec.loader.exec_module(upstream)
    # Forward URI actions to the owned instance, never spawn another proxy.
    if upstream.CLI_ARGS.uri or upstream.CLI_ARGS.open_ui:
        from uri_scheme import parse_uri, signal_running_instance
        params = parse_uri(upstream.CLI_ARGS.uri or "idvlogin://open")
        if not signal_running_instance(params.get("game_id", ""), params.get("action", "open"), params.get("installation_id", ""), params.get("view", "")):
            raise RuntimeError("请先从助手启动登录代理")
        return
    import logutil
    from prefetch_context import in_prefetch
    logutil.logger.add(lambda message: emit(message.record["level"].name, message.record["message"]), level="INFO", filter=lambda record: not in_prefetch())
    import certmgr
    certmgr.certmgr.import_to_root = import_certificate
    adapt(upstream)

    def watch_host():
        # The pipe closes if the assistant crashes, too. No console is needed.
        sys.stdin.readline()
        try:
            stage("stopping")
            emit("INFO", "收到助手停止请求，正在清理登录代理")
            upstream.handle_exit()
        finally:
            os._exit(0)

    threading.Thread(target=watch_host, daemon=True).start()
    stage("initializing")
    emit("INFO", "正在初始化官方登录组件（后台模式）")
    upstream.main(upstream.CLI_ARGS)


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    try:
        main()
    except Exception as error:
        emit("CRITICAL", f"登录组件启动失败：{error}")
        raise SystemExit(1)
