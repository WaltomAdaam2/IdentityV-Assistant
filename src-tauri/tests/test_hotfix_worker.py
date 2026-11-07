"""Guard against accidental invocation of the retired helper by old integrations."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class RetiredHotfixTests(unittest.TestCase):
    def test_retired_worker_never_changes_component_files(self):
        worker = Path(__file__).parents[1] / "src/launcher/hotfix_worker.py"
        with tempfile.TemporaryDirectory() as directory:
            original = Path(directory) / "config.json"
            original.write_text('{"accounts":"preserved"}', encoding="utf-8")
            result = subprocess.run([sys.executable, str(worker.resolve())], cwd=directory,
                input='{"hotfixes":[{"target_module":"main"}]}', text=True, capture_output=True, timeout=5)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("disabled", result.stderr)
            self.assertEqual(original.read_text(encoding="utf-8"), '{"accounts":"preserved"}')
            self.assertEqual(list(Path(directory).iterdir()), [original])
