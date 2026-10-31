"""Apply an already reviewed snapshot through the official hotfix implementation."""
import json
import os
from pathlib import Path
import re
import sys
import threading
import time
import types


class UpdateCancelled(RuntimeError):
    pass


def download_source(url, fallbacks, report, cancelled):
    """Keep the official byte-return contract while reporting actual transfer."""
    def check_cancelled():
        if cancelled is not None and cancelled.is_set():
            raise UpdateCancelled("组件更新已取消")

    check_cancelled()
    import requests
    from ssl_utils import should_verify_ssl
    with requests.Session() as session:
        session.trust_env = False
        error = "Download failed"
        for candidate in [url, *fallbacks]:
            check_cancelled()
            try:
                with session.get(candidate, timeout=20, stream=True,
                                 headers={"Accept": "text/plain, */*", "Accept-Encoding": "identity"},
                                 verify=should_verify_ssl()) as response:
                    if response.status_code != 200:
                        return False, b"", f"HTTP {response.status_code}"
                    length = response.headers.get("Content-Length", "")
                    total = int(length) if length.isdigit() and response.headers.get("Content-Encoding", "identity") == "identity" else None
                    content = bytearray()
                    report("downloading", 0, total)
                    last_report = time.monotonic()
                    for chunk in response.iter_content(chunk_size=16 * 1024):
                        check_cancelled()
                        content.extend(chunk)
                        if len(content) > 4 * 1024 * 1024:
                            raise RuntimeError("热更新源文件超过 4 MiB 限制")
                        if time.monotonic() - last_report >= 0.1:
                            report("downloading", len(content), total)
                            last_report = time.monotonic()
                    check_cancelled()
                    report("downloading", len(content), total)
                    report("applying", len(content), total)
                    return True, bytes(content), ""
            except UpdateCancelled:
                raise
            except Exception as exc:
                error = str(exc)
        return False, b"", error


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
    # The pinned runtime normally tolerates cache write failures. Require an
    # actual atomic write before replacing any module, even after an old probe.
    try:
        genv.set("hotfix_probed", True, True, require_saved=True)
    except OSError as error:
        raise RuntimeError("无法写入组件更新记录，请检查组件数据目录的写入权限后重试") from error
    # Keep official backup/compile/replace/pending/rollback handling. Constrain
    # resolution to the reviewed runtime, and honor the selected download source.
    hotfixmgr._resolve_module_target_path = lambda module: (targets[module], "pyc")
    current = {}
    def source_info(module, commit):
        current.update(module=module, commit=commit)
        return prefix + f"https://raw.githubusercontent.com/KKeygen/idv-login/{commit}/src/{module.replace('.', '/')}.py", []
    def report(phase, downloaded, total):
        print("IDV_UPDATE_PROGRESS " + json.dumps({**current, "phase": phase,
              "downloaded": downloaded, "total": total}), flush=True)
    hotfixmgr._build_remote_source_info = source_info
    hotfixmgr._download_text = lambda url, fallbacks: download_source(url, fallbacks, report, cancelled)
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
    # In-memory success is insufficient: upstream cached writes may silently
    # fail. Preserve its records and verify that the next process can read them.
    try:
        saved = json.loads((data_dir / "config.json").read_text(encoding="utf-8-sig"))
        saved_records = saved.get("hotfix_records", {})
        saved_pending = saved.get("hotfix_pending_validate", [])
        if isinstance(saved_pending, str):
            saved_pending = [saved_pending]
        if not isinstance(saved_records, dict) or not isinstance(saved_pending, list):
            raise ValueError("Invalid validation records")
        if any(saved_records.get(hotfixmgr._make_id(item)) != records.get(hotfixmgr._make_id(item)) for item in items):
            raise ValueError("Update records were not persisted")
        if pending and (any(key not in saved_pending for key in pending) or saved.get("last_run_state") != "restart"):
            raise ValueError("Validation bookkeeping was not persisted")
    except (OSError, ValueError, AttributeError) as error:
        raise RuntimeError("组件热更新记录未保存，无法确认更新成功；请检查数据目录权限并重新检查更新") from error


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
