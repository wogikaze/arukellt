"""Unit tests for docs_domain checks (stdlib unittest, no pytest)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

# Ensure scripts/ is on path
_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from docs_domain.checks import (  # noqa: E402
    run_consistency,
    run_examples,
    run_freshness,
    run_regenerate,
)

_ROOT = Path("/home/wogikaze/arukellt")


class TestDryRun(unittest.TestCase):
    def _assert_dry(self, fn, *args, **kwargs):
        rc, out = fn(_ROOT, dry_run=True, *args, **kwargs)
        self.assertEqual(rc, 0)
        self.assertEqual(out, "")

    def test_consistency_dry(self):
        self._assert_dry(run_consistency)

    def test_freshness_dry(self):
        self._assert_dry(run_freshness)

    def test_examples_dry(self):
        self._assert_dry(run_examples)

    def test_regenerate_dry(self):
        self._assert_dry(run_regenerate)

    def test_regenerate_check_only_dry(self):
        self._assert_dry(run_regenerate, check_only=True)


class TestCommands(unittest.TestCase):
    """Verify the correct commands are assembled (no actual subprocess)."""

    def _captured_cmd(self, fn, *args, **kwargs):
        with patch("docs_domain.checks.subprocess.run") as mock_run:
            mock_result = MagicMock()
            mock_result.returncode = 0
            mock_result.stdout = "ok"
            mock_run.return_value = mock_result
            fn(_ROOT, dry_run=False, *args, **kwargs)
            return mock_run.call_args[0][0]

    def test_consistency_cmd(self):
        cmd = self._captured_cmd(run_consistency)
        self.assertEqual(cmd, ["python3", "scripts/check/check-docs-consistency.py"])

    def test_freshness_cmd(self):
        cmd = self._captured_cmd(run_freshness)
        self.assertEqual(cmd, ["python3", "scripts/check/check-docs-freshness.py"])

    def test_examples_cmd(self):
        cmd = self._captured_cmd(run_examples)
        self.assertEqual(cmd, ["python3", "scripts/check/check-doc-examples.py", "docs/"])

    def test_regenerate_cmd_no_check(self):
        cmd = self._captured_cmd(run_regenerate, check_only=False)
        self.assertEqual(cmd, ["python3", "scripts/gen/generate-docs.py"])

    def test_regenerate_cmd_check_only(self):
        cmd = self._captured_cmd(run_regenerate, check_only=True)
        self.assertEqual(cmd, ["python3", "scripts/gen/generate-docs.py", "--check"])


if __name__ == "__main__":
    unittest.main()
