"""Focused tests for deterministic CQ-13 metrics and baseline handling."""

import tempfile
import unittest
from dataclasses import asdict
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
from scripts.quality.baseline import (
    BaselineError,
    InventoryMetadata,
    MetricsMetadata,
    QualityBaseline,
    read_baseline,
    with_inventory,
    write_baseline,
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


class TestWrapperClassification(unittest.TestCase):
    def _category(self, body: str, path: str = "src/compiler/lower/helper.ark") -> str:
        from scripts.quality.debt import classify_wrappers

        return classify_wrappers(path, body)[0].category

    def test_exact_same_argument_forwarder(self):
        self.assertEqual(
            self._category("fn alias(a: I32, b: String) -> I32 {\n    return impl::alias(a, b)\n}\n"),
            "pure_forwarder",
        )

    def test_reordered_default_clone_and_conversion_are_semantic(self):
        bodies = (
            "fn f(a: I32, b: I32) {\n    impl::f(b, a)\n}\n",
            "fn f(a: I32) {\n    impl::f(a, 0)\n}\n",
            "fn f(a: String) {\n    impl::f(clone(a))\n}\n",
            "fn f(a: I32) {\n    impl::f(i32_to_i64(a))\n}\n",
        )
        for body in bodies:
            with self.subTest(body=body):
                self.assertEqual(self._category(body), "semantic_wrapper")

    def test_validation_is_semantic(self):
        body = "fn f(a: I32) {\n    if a < 0 { return }\n    impl::f(a)\n}\n"
        self.assertEqual(self._category(body), "semantic_wrapper")

    def test_facade_constructor_and_accessor_are_preserved(self):
        facade = "pub fn run(a: I32) {\n    impl::run(a)\n}\n"
        constructor = "fn Request_new(a: I32) {\n    Request { value: a }\n}\n"
        accessor = "fn request_value(a: Request) {\n    a.value\n}\n"
        self.assertEqual(self._category(facade), "boundary_facade")
        self.assertEqual(self._category(constructor), "record_accessor_or_constructor")
        self.assertEqual(self._category(accessor), "record_accessor_or_constructor")

    def test_comments_and_strings_do_not_create_forwarders(self):
        body = '''fn f(a: I32) {
    let sample = "impl::f(a)"
    // impl::f(a)
    return a
}
'''
        self.assertNotEqual(self._category(body), "pure_forwarder")

    def test_unused_wrapper_only_file_is_reported(self):
        from scripts.quality.debt import collect_wrapper_debt

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "src/compiler/lower"
            compiler.mkdir(parents=True)
            (compiler / "implementation.ark").write_text(
                "fn run(a: I32) {\n    return a\n}\n", encoding="utf-8"
            )
            (compiler / "forwarder.ark").write_text(
                "use lower::implementation\nfn run(a: I32) {\n    implementation::run(a)\n}\n",
                encoding="utf-8",
            )
            inventory = collect_wrapper_debt(root)
            self.assertEqual(len(inventory.unjustified_pure_forwarders), 1)
            self.assertEqual(
                inventory.wrapper_only_single_function_files,
                ("src/compiler/lower/forwarder.ark",),
            )


class TestMetricsBaseline(unittest.TestCase):
    def _repository(self, root: Path) -> None:
        compiler = root / "src/compiler"
        compiler.mkdir(parents=True)
        (compiler / "main.ark").write_text("fn main() {}\n", encoding="utf-8")
        (root / "issues/open").mkdir(parents=True)
        (root / "issues/open/9-metrics.md").write_text("# metrics\n", encoding="utf-8")

    def _seed_baseline(self, root: Path):
        report = collect_metrics(root)
        baseline = QualityBaseline(
            inventory=InventoryMetadata("tooling", True, 8, 64, 200),
            counts={
                "tabs_files": 0,
                "extreme_indent_lines": 0,
                "lines_ge_200": 4,
                "thin_wrappers": 3,
                "single_function_files": 2,
            },
            metrics_metadata=MetricsMetadata(8, "string metadata is valid"),
            metrics={
                name: asdict(report.distributions[name]) for name in METRIC_NAMES
            },
        )
        path = root / "docs/data/ark-code-quality-baseline.toml"
        write_baseline(path, baseline, METRIC_NAMES)
        return path, report

    def test_baseline_write_read_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._repository(root)
            baseline_path, report = self._seed_baseline(root)
            write_metrics_baseline(root, report, 9, "initial fixture baseline")
            baseline = _baseline_metrics(baseline_path)
            self.assertEqual(set(baseline), set(METRIC_NAMES))
            self.assertEqual(
                read_baseline(baseline_path, METRIC_NAMES).counts["lines_ge_200"],
                4,
            )

    def test_report_does_not_implicitly_update_baseline(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._repository(root)
            baseline_path, _ = self._seed_baseline(root)
            before = baseline_path.read_bytes()
            self.assertEqual(run_metrics_report(root), 0)
            self.assertEqual(baseline_path.read_bytes(), before)

    def test_inventory_update_preserves_metrics_and_only_lowers(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._repository(root)
            baseline_path, _ = self._seed_baseline(root)
            baseline = read_baseline(baseline_path, METRIC_NAMES)
            lowered = dict(baseline.counts)
            lowered["lines_ge_200"] = 3
            write_baseline(
                baseline_path,
                with_inventory(baseline, lowered, 9),
                METRIC_NAMES,
            )
            updated = read_baseline(baseline_path, METRIC_NAMES)
            self.assertEqual(updated.metrics, baseline.metrics)
            self.assertEqual(updated.metrics_metadata, baseline.metrics_metadata)
            raised = dict(lowered)
            raised["lines_ge_200"] = 5
            with self.assertRaisesRegex(BaselineError, "only lower counts"):
                with_inventory(updated, raised, 9)

    def test_malformed_type_and_missing_key_are_explained(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "baseline.toml"
            cases = (
                ("not = [valid", "malformed baseline TOML"),
                ("[inventory]\nowner = 4\n", "owner must be str"),
                ('[inventory]\nowner = "tooling"\n', "missing required key"),
            )
            for content, message in cases:
                with self.subTest(message=message):
                    path.write_text(content, encoding="utf-8")
                    with self.assertRaisesRegex(BaselineError, message):
                        read_baseline(path, METRIC_NAMES)


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

    def test_missing_formatter_and_linter_are_failures(self):
        from scripts.quality.checks import ToolResult, run_fmt, run_lint

        missing = [ToolResult("sample.ark", ("missing-tool",), 127, "not found")]
        with mock.patch(
            "scripts.quality.checks.ark_paths", return_value=["sample.ark"]
        ), mock.patch("scripts.quality.checks._run_parallel", return_value=missing):
            self.assertEqual(run_fmt(Path("/tmp"), [], True, False, False), 1)
            self.assertEqual(run_lint(Path("/tmp"), [], False, False, False), 1)

    def test_lint_command_includes_smoke_and_propagates_exit_code(self):
        from scripts.quality.checks import run_lint_command

        with mock.patch("scripts.quality.checks.run_lint", return_value=0), mock.patch(
            "scripts.quality.checks._run_command", return_value=1
        ) as command:
            self.assertEqual(run_lint_command(Path("/tmp"), [], False, False, False), 1)
            self.assertIn("scripts/check/check-ark-lint-smoke.py", command.call_args.args[1])

    def test_enforced_sources_cannot_succeed_with_zero_selected_files(self):
        from scripts.quality.checks import run_fmt, run_lint

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "src/compiler").mkdir(parents=True)
            (root / "src/compiler/main.ark").write_text("fn main() {}\n", encoding="utf-8")
            (root / "docs/data").mkdir(parents=True)
            (root / "docs/data/tooling-inventory.toml").write_text(
                '[[families]]\next = ".ark"\nroots = ["src/compiler/"]\n',
                encoding="utf-8",
            )
            with mock.patch("scripts.quality.checks.ark_paths", return_value=[]):
                self.assertEqual(run_fmt(root, [], True, False, False), 1)
                self.assertEqual(run_lint(root, [], False, False, False), 1)


if __name__ == "__main__":
    unittest.main()
