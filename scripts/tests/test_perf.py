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
from util.benchmark_runner import (  # noqa: E402
    comparison_gaps,
    measurement_failures,
    rel,
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
            "--output-md", "docs/history/benchmarks/benchmark-results.md",
        ])

    def test_gate_cmd_update(self):
        cmd = self._captured_cmd(run_gate, update=True)
        self.assertEqual(cmd, [
            sys.executable, "scripts/util/benchmark_runner.py",
            "--mode", "update-baseline",
            "--baseline", "tests/baselines/perf/baselines.json",
            "--output-json", "tests/baselines/perf/current.json",
            "--output-md", "docs/history/benchmarks/benchmark-results.md",
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
            "--output-md", "docs/history/benchmarks/benchmark-results.md",
        ])

    def test_benchmarks_cmd_no_quick(self):
        cmd = self._captured_cmd(run_benchmarks, quick=False)
        self.assertEqual(cmd, [
            sys.executable, "scripts/util/benchmark_runner.py",
            "--mode", "full",
            "--output-json", "tests/baselines/perf/current.json",
            "--output-md", "docs/history/benchmarks/benchmark-results.md",
        ])


class TestPathRendering(unittest.TestCase):
    def test_rel_accepts_repository_relative_paths(self):
        self.assertEqual(rel(Path("tests/baselines/perf/baselines.json")),
                         "tests/baselines/perf/baselines.json")

    def test_rel_accepts_absolute_repository_paths(self):
        path = (
            Path(__file__).resolve().parents[2]
            / "tests"
            / "baselines"
            / "perf"
            / "baselines.json"
        )
        self.assertEqual(rel(path), "tests/baselines/perf/baselines.json")


class TestMeasurementGate(unittest.TestCase):
    def test_compile_failure_is_reported(self):
        current = {
            "benchmarks": [
                {
                    "name": "fib",
                    "metrics": ["compile", "runtime"],
                    "compile": {
                        "status": "error",
                        "stderr_head": ["failed to read file"],
                    },
                    "runtime": {"status": "skipped"},
                }
            ]
        }
        self.assertEqual(
            measurement_failures(current),
            ["fib: compile error: failed to read file"],
        )

    def test_runtime_failure_is_reported_after_successful_compile(self):
        current = {
            "benchmarks": [
                {
                    "name": "fib",
                    "metrics": ["compile", "runtime"],
                    "compile": {"status": "ok"},
                    "runtime": {"status": "error", "reason": "trap"},
                }
            ]
        }
        self.assertEqual(measurement_failures(current), ["fib: runtime error: trap"])

    def test_runtime_correctness_failure_is_reported(self):
        current = {
            "benchmarks": [
                {
                    "name": "fib",
                    "metrics": ["compile", "runtime"],
                    "compile": {"status": "ok"},
                    "runtime": {"status": "ok", "correctness": "fail"},
                }
            ]
        }
        self.assertEqual(measurement_failures(current), ["fib: runtime correctness fail"])

    def test_missing_baseline_metric_is_reported(self):
        comparison = {
            "baseline_available": True,
            "benchmarks": [
                {
                    "name": "new_fixture",
                    "metrics": {
                        "compile_ms": {"status": "skipped"},
                        "run_ms": {"status": "pass"},
                    },
                }
            ],
        }
        self.assertEqual(comparison_gaps(comparison), ["new_fixture: compile_ms"])


if __name__ == "__main__":
    unittest.main()
