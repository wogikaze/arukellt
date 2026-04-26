"""Unit tests for perf domain checks (stdlib unittest, no pytest)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

# Ensure scripts/ is on path
_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from perf.checks import (  # noqa: E402
    run_baseline,
    run_benchmarks,
    run_gate,
)

_ROOT = Path("/home/wogikaze/arukellt")


class TestDryRun(unittest.TestCase):
    def _assert_dry(self, fn, *args, **kwargs):
        rc, out = fn(_ROOT, dry_run=True, *args, **kwargs)
        self.assertEqual(rc, 0)
        self.assertEqual(out, "")

    def test_gate_dry(self):
        self._assert_dry(run_gate)

    def test_gate_update_dry(self):
        self._assert_dry(run_gate, update=True)

    def test_baseline_dry(self):
        self._assert_dry(run_baseline)

    def test_benchmarks_dry(self):
        self._assert_dry(run_benchmarks)

    def test_benchmarks_no_quick_dry(self):
        self._assert_dry(run_benchmarks, quick=False)


class TestCommands(unittest.TestCase):
    """Verify the correct commands are assembled (no actual subprocess)."""

    def _captured_cmd(self, fn, *args, **kwargs):
        with patch("perf.checks.subprocess.run") as mock_run:
            mock_result = MagicMock()
            mock_result.returncode = 0
            mock_result.stdout = "ok"
            mock_run.return_value = mock_result
            fn(_ROOT, dry_run=False, *args, **kwargs)
            return mock_run.call_args[0][0]

    def test_gate_cmd_no_update(self):
        cmd = self._captured_cmd(run_gate, update=False)
        self.assertEqual(cmd, [
            sys.executable, "scripts/util/benchmark_runner.py",
            "--mode", "ci",
            "--baseline", "tests/baselines/perf/baselines.json",
            "--output-json", "tests/baselines/perf/current.json",
            "--output-md", "docs/process/benchmark-results.md",
        ])

    def test_gate_cmd_update(self):
        cmd = self._captured_cmd(run_gate, update=True)
        self.assertEqual(cmd, [
            sys.executable, "scripts/util/benchmark_runner.py",
            "--mode", "update-baseline",
            "--baseline", "tests/baselines/perf/baselines.json",
            "--output-json", "tests/baselines/perf/current.json",
            "--output-md", "docs/process/benchmark-results.md",
        ])

    def test_baseline_cmd(self):
        cmd = self._captured_cmd(run_baseline)
        self.assertEqual(cmd, [sys.executable, "scripts/util/collect-baseline.py"])

    def test_benchmarks_cmd_quick(self):
        cmd = self._captured_cmd(run_benchmarks, quick=True)
        self.assertEqual(cmd, [
            sys.executable, "scripts/util/benchmark_runner.py",
            "--mode", "quick",
            "--output-json", "tests/baselines/perf/current.json",
            "--output-md", "docs/process/benchmark-results.md",
        ])

    def test_benchmarks_cmd_no_quick(self):
        cmd = self._captured_cmd(run_benchmarks, quick=False)
        self.assertEqual(cmd, [
            sys.executable, "scripts/util/benchmark_runner.py",
            "--mode", "full",
            "--output-json", "tests/baselines/perf/current.json",
            "--output-md", "docs/process/benchmark-results.md",
        ])


if __name__ == "__main__":
    unittest.main()
