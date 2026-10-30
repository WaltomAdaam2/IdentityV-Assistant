import importlib.util
import os
from pathlib import Path
import sys
import subprocess
import time
import tempfile
import types
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location("worker", Path(__file__).parents[1] / "src/launcher/hotfix_worker.py")
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)


class HotfixWorkerTests(unittest.TestCase):
    def test_worker_stdin_cancel_finishes_before_mutating_a_module(self):
        import json
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "fixture.pyc").write_bytes(b"unchanged")
            (root / "buildinfo.py").write_text("VERSION = 'v6.3.2-stable'", encoding="utf-8")
            (root / "envmgr.py").write_text("class genv:\n    set = staticmethod(lambda *args: None)\n", encoding="utf-8")
            (root / "hotfixmgr.py").write_text("import time\nfrom pathlib import Path\n"
                "def _download_text(*args):\n    Path('started').write_text('ready')\n    from time import sleep\n    sleep(0.5)\n    return True, b'new', ''\n"
                "def handle_if_needed(snapshot):\n    _download_text('url', [])\n    Path('fixture.pyc').write_bytes(b'changed')\n"
                "def _get_records(): return {}\n", encoding="utf-8")
            process = subprocess.Popen([sys.executable, "-I", "-B", str(worker.__file__), str(root), str(root)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", creationflags=0x08000000)
            try:
                process.stdin.write(json.dumps({"source": "direct", "current_version": "v6.3.2-stable", "hotfixes": [{"target_module": "fixture", "target_commit": "abcdef1234567"}]}) + "\n")
                process.stdin.flush()
                deadline = time.monotonic() + 10
                while not (root / "started").exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue((root / "started").exists())
                output, errors = process.communicate("cancel\n", timeout=5)
                self.assertIn("组件更新已取消", output, errors)
                self.assertEqual((root / "fixture.pyc").read_bytes(), b"unchanged")
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()

    def test_applies_only_confirmed_items_using_selected_source_and_owned_target(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "fixture.pyc").write_bytes(b"fixture")
            item = {"target_module": "fixture", "target_commit": "abcdef1234567", "note": "test"}
            hotfix = types.SimpleNamespace(_get_records=lambda: {"id": {"status": "pending_validate"}}, _make_id=lambda item: "id", _set_pending_ids=Mock())
            observed = []
            def handle(snapshot):
                observed.extend(snapshot.get_hotfixes())
                self.assertEqual(hotfix._resolve_module_target_path("fixture"), (str((root / "fixture.pyc").resolve()), "pyc"))
                self.assertEqual(hotfix._build_remote_source_info("fixture", item["target_commit"]), ("https://gh-proxy.com/https://raw.githubusercontent.com/KKeygen/idv-login/abcdef1234567/src/fixture.py", []))
            hotfix.handle_if_needed = handle
            modules = {"buildinfo": types.SimpleNamespace(VERSION="v6.3.2-stable"), "hotfixmgr": hotfix, "envmgr": types.SimpleNamespace(genv=Mock())}
            cwd = Path.cwd()
            try:
                with patch.dict(sys.modules, modules):
                    worker.apply(root, root, {"source": "proxy", "current_version": "v6.3.2-stable", "hotfixes": [item]})
                    self.assertEqual(observed, [item])
                    with self.assertRaises(RuntimeError):
                        worker.apply(root, root, {"source": "direct", "current_version": "v6.3.2-stable", "hotfixes": [{**item, "target_module": "../escape"}]})
            finally:
                os.chdir(cwd)

    @unittest.skipUnless(os.environ.get("IDV_OFFICIAL_ROOT"), "official runtime is optional")
    def test_official_hotfix_implementation_compiles_and_records_a_harmless_module(self):
        import py_compile
        official = Path(os.environ["IDV_OFFICIAL_ROOT"])
        sys.path.insert(0, str(official / "src"))
        import hotfixmgr
        cwd = Path.cwd()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            module = root / "fixture.py"
            module.write_text("value = 'before'", encoding="utf-8")
            py_compile.compile(str(module), cfile=str(root / "fixture.pyc"), doraise=True)
            try:
                with patch.object(hotfixmgr, "_download_text", return_value=(True, b"value = 'after'", "")), patch.object(hotfixmgr.time, "sleep"):
                    worker.apply(root, root, {"source": "direct", "current_version": "v6.3.2-stable", "hotfixes": [{"target_module": "fixture", "target_commit": "abcdef1234567"}]})
                loaded = importlib.util.spec_from_file_location("updated_fixture", root / "fixture.pyc")
                updated = importlib.util.module_from_spec(loaded)
                loaded.loader.exec_module(updated)
                self.assertEqual(updated.value, "after")
                self.assertTrue((root / "fixture.pyc.bak").exists())
                before_cancel = (root / "fixture.pyc").read_bytes()
                cancelled = worker.threading.Event()
                cancelled.set()
                with patch.object(hotfixmgr, "_download_text") as download:
                    with self.assertRaisesRegex(RuntimeError, "取消"):
                        worker.apply(root, root, {"source": "direct", "current_version": "v6.3.2-stable", "hotfixes": [{"target_module": "fixture", "target_commit": "abcdef2234567"}]}, cancelled)
                    download.assert_not_called()
                self.assertEqual((root / "fixture.pyc").read_bytes(), before_cancel)
                self.assertIn("v6.3.2-stable|fixture@abcdef1234567", hotfixmgr._get_pending_ids())
            finally:
                os.chdir(cwd)


if __name__ == "__main__":
    unittest.main()
