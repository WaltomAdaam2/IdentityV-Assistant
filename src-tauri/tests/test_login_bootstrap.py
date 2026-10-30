import importlib.util
from pathlib import Path
import types
import os
import tempfile
import sys
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location("bootstrap", Path(__file__).parents[1] / "src/launcher/bootstrap.py")
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)


class ManagedLoginTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get("IDV_OFFICIAL_ROOT"), "official runtime is optional")
    def test_adapter_loads_real_official_modules_without_starting_login(self):
        official = Path(os.environ["IDV_OFFICIAL_ROOT"])
        original_spec = importlib.util.spec_from_file_location
        calls = []

        def load_spec(name, location, *args, **kwargs):
            spec = original_spec(name, location, *args, **kwargs)
            if name == "main":
                original_exec = spec.loader.exec_module
                def load(module):
                    original_exec(module)
                    def check(cli):
                        self.assertIs(sys.modules["main"], module)
                        self.assertEqual(cli.proxy_port, 12345)
                        self.assertEqual(module.setup_shortcuts.__name__, "skip")
                        self.assertEqual(module.hotfixmgr.restart_self.__name__, "reject_restart")
                        calls.append(True)
                    module.main = check
                spec.loader.exec_module = load
            return spec

        cwd = Path.cwd()
        with tempfile.TemporaryDirectory() as data:
            try:
                with patch.dict(os.environ, {"IDV_ASSISTANT_SRC": str(official / "src"), "PROGRAMDATA": data}), patch.object(sys, "argv", [str(bootstrap.__file__), "--proxy-port", "12345"]), patch.object(importlib.util, "spec_from_file_location", side_effect=load_spec), patch.object(threading := bootstrap.threading.Thread, "start"), patch.object(bootstrap, "emit"):
                    bootstrap.main()
                self.assertEqual(calls, [True])
            finally:
                if "logutil" in sys.modules:
                    sys.modules["logutil"].logger.remove()
                os.chdir(cwd)

    def test_managed_start_skips_desktop_update_and_restart(self):
        upstream = types.SimpleNamespace(hotfixmgr=types.SimpleNamespace(), genv=Mock())
        with patch.object(bootstrap, "emit"):
            bootstrap.adapt(upstream)
            upstream.setup_shortcuts()
            upstream.handle_update()
            upstream.hotfixmgr.handle_if_needed(None)
            upstream.hotfixmgr.install_import_hook()
            with self.assertRaises(SystemExit):
                upstream.hotfixmgr.restart_self("update")
            with self.assertRaises(SystemExit):
                upstream.handle_error_and_exit(RuntimeError("fixture"))
        upstream.genv.set.assert_any_call("last_run_state", "crash", True)

    def test_certificate_failure_raises_instead_of_returning_false_for_pause(self):
        with patch.object(bootstrap.subprocess, "run", return_value=types.SimpleNamespace(returncode=1)) as run:
            with self.assertRaises(RuntimeError):
                bootstrap.import_certificate(None, "fixture.pem")
            self.assertEqual(run.call_args.kwargs["creationflags"], 0x08000000)

    def test_logs_filter_debug_preserve_severity_and_redact_credentials(self):
        import io, json
        output = io.StringIO()
        with patch.object(bootstrap, "OUTPUT", output):
            bootstrap.emit("DEBUG", "ignore")
            bootstrap.emit("CRITICAL", 'token=secret password="two words" https://example.test/?ticket=hidden')
        record = json.loads(output.getvalue().split("IDV_ASSISTANT_LOG ", 1)[1])
        self.assertEqual(record["level"], "CRITICAL")
        self.assertNotIn("secret", record["message"])
        self.assertNotIn("two words", record["message"])
        self.assertNotIn("hidden", record["message"])


if __name__ == "__main__":
    unittest.main()
