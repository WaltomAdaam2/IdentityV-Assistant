"""Apply an already reviewed snapshot through the official hotfix implementation."""
import json
import os
from pathlib import Path
import re
import sys
import threading
import time
import types


def apply(source_dir, data_dir, request, cancelled=None):
    sys.path.insert(0, str(source_dir))
    os.chdir(data_dir)
    import buildinfo
    import hotfixmgr
    from envmgr import genv
    if buildinfo.VERSION != request["current_version"]:
        raise RuntimeError("组件版本已变化，请重新检查更新")
    genv.set("VERSION", buildinfo.VERSION)
    genv.set("SCRIPT_DIR", str(source_dir))
    genv.set("FP_WORKDIR", str(data_dir))
    prefixes = {"direct": "", "proxy": "https://gh-proxy.com/", "ghfast": "https://ghfast.top/", "ghproxy_net": "https://ghproxy.net/"}
    prefix = prefixes[request["source"]]
    items = request["hotfixes"]
    targets = {}
    for item in items:
        module, commit = item["target_module"], item["target_commit"]
        if not re.fullmatch(r"[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*", module) or not re.fullmatch(r"[0-9a-fA-F]{7,40}", commit):
            raise RuntimeError("热更新模块或提交无效")
        target = (source_dir / (module.replace(".", "/") + ".pyc")).resolve()
        if not target.is_relative_to(source_dir.resolve()) or not target.is_file():
            raise RuntimeError("热更新目标不在组件目录内")
        targets[module] = str(target)
    # Keep official backup/compile/replace/pending/rollback handling. Constrain
    # resolution to the reviewed runtime, and honor the selected download source.
    hotfixmgr._resolve_module_target_path = lambda module: (targets[module], "pyc")
    hotfixmgr._build_remote_source_info = lambda module, commit: (prefix + f"https://raw.githubusercontent.com/KKeygen/idv-login/{commit}/src/{module.replace('.', '/')}.py", [])
    if cancelled is not None:
        original_download = hotfixmgr._download_text
        def download(*args):
            if cancelled.is_set():
                raise RuntimeError("组件更新已取消")
            result = original_download(*args)
            if cancelled.is_set():
                raise RuntimeError("组件更新已取消")
            return result
        hotfixmgr._download_text = download
    # Confirmation already happened in the assistant dialog.
    hotfixmgr.time = types.SimpleNamespace(time=time.time, sleep=lambda seconds: None)
    hotfixmgr.restart_self = lambda reason="": genv.set("last_run_state", "restart", True)
    try:
        hotfixmgr.handle_if_needed(types.SimpleNamespace(get_hotfixes=lambda: items))
    finally:
        # Cancellation can interrupt a later download after an earlier module
        # was replaced. Preserve official validation/rollback bookkeeping.
        pending = [key for key, record in hotfixmgr._get_records().items() if isinstance(record, dict) and record.get("status") == "pending_validate"]
        if pending:
            hotfixmgr._set_pending_ids(pending)
            genv.set("last_run_state", "restart", True)
    records = hotfixmgr._get_records()
    failures = [item["target_module"] + ": " + str(records.get(hotfixmgr._make_id(item), {}).get("error", "未完成")) for item in items if records.get(hotfixmgr._make_id(item), {}).get("status") not in {"pending_validate", "applied"}]
    if failures:
        raise RuntimeError("部分热更新未应用：" + ", ".join(failures))


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    try:
        request = json.loads(sys.stdin.readline())
        cancelled = threading.Event()
        threading.Thread(target=lambda: (sys.stdin.readline(), cancelled.set()), daemon=True).start()
        apply(Path(sys.argv[1]), Path(sys.argv[2]), request, cancelled)
        print("IDV_UPDATE_RESULT " + json.dumps({"ok": True}), flush=True)
    except Exception as error:
        print("IDV_UPDATE_RESULT " + json.dumps({"ok": False, "error": str(error)}, ensure_ascii=False), flush=True)
        raise SystemExit(1)
