"""Focused tests for deterministic CQ-13 metrics and baseline handling."""

import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts.quality.metrics import (
    METRIC_NAMES,
    _baseline_metrics,
    collect_metrics,
    run_metrics_report,
    sanitize_ark_lines,
    scan_ark_source,
    write_metrics_baseline,
)


class TestMetricsScanner(unittest.TestCase):
    def test_strings_and_comments_do_not_add_braces_or_branches(self):
        text = '''fn stable(input: I32) -> I32 {
    let sample = "if { while }"
    // if value { value }
    /* match { => } */
    return input
}
'''
        sanitized = "\n".join(sanitize_ark_lines(text))
        self.assertNotIn("while", sanitized)
        _, functions = scan_ark_source("src/compiler/stable.ark", text)
        self.assertEqual(functions[0].complexity, 1)
        self.assertEqual(functions[0].max_nesting_depth, 0)

    def test_collection_is_deterministic_without_git_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "src/compiler"
            compiler.mkdir(parents=True)
            (compiler / "b.ark").write_text("fn b() {}\n", encoding="utf-8")
            (compiler / "a.ark").write_text("use b\nfn a() {\n    if true { return }\n}\n", encoding="utf-8")
            first = collect_metrics(root)
            second = collect_metrics(root)
            self.assertEqual(first.to_dict(), second.to_dict())
            self.assertFalse(first.churn_available)
            self.assertTrue(all(item.churn is None for item in first.files))


class TestMetricsBaseline(unittest.TestCase):
    def _repository(self, root: Path) -> None:
        compiler = root / "src/compiler"
        compiler.mkdir(parents=True)
        (compiler / "main.ark").write_text("fn main() {}\n", encoding="utf-8")
        (root / "issues/open").mkdir(parents=True)
        (root / "issues/open/9-metrics.md").write_text("# metrics\n", encoding="utf-8")

    def test_baseline_write_read_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._repository(root)
            report = collect_metrics(root)
            write_metrics_baseline(root, report, 9, "initial fixture baseline")
            baseline = _baseline_metrics(root / "docs/data/ark-code-quality-baseline.toml")
            self.assertEqual(set(baseline), set(METRIC_NAMES))

    def test_report_does_not_implicitly_update_baseline(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._repository(root)
            report = collect_metrics(root)
            write_metrics_baseline(root, report, 9, "initial fixture baseline")
            baseline_path = root / "docs/data/ark-code-quality-baseline.toml"
            before = baseline_path.read_bytes()
            self.assertEqual(run_metrics_report(root), 0)
            self.assertEqual(baseline_path.read_bytes(), before)


class TestCanonicalToolExitContracts(unittest.TestCase):
    def test_fmt_check_propagates_pass_and_failure(self):
        from scripts.quality.checks import ToolResult, run_fmt

        for returncode in (0, 1):
            with self.subTest(returncode=returncode), mock.patch(
                "scripts.quality.checks.ark_paths", return_value=["sample.ark"]
            ), mock.patch(
                "scripts.quality.checks._run_parallel",
                return_value=[ToolResult("sample.ark", ("fmt",), returncode, "")],
            ):
                self.assertEqual(run_fmt(Path("/tmp"), [], True, False, False), returncode)

    def test_lint_command_includes_smoke_and_propagates_exit_code(self):
        from scripts.quality.checks import run_lint_command

        with mock.patch("scripts.quality.checks.run_lint", return_value=0), mock.patch(
            "scripts.quality.checks._run_command", return_value=1
        ) as command:
            self.assertEqual(run_lint_command(Path("/tmp"), [], False, False, False), 1)
            self.assertIn("scripts/check/check-ark-lint-smoke.py", command.call_args.args[1])


if __name__ == "__main__":
    unittest.main()
