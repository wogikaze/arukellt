"""Unit tests for measure-native-cpp-fixture-coverage diagnostic helpers."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts/check/measure-native-cpp-fixture-coverage.py"


def load_measure():
    spec = importlib.util.spec_from_file_location(
        "measure_native_cpp_fixture_coverage", MODULE_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {MODULE_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class PrimaryDiagnosticSelectionTests(unittest.TestCase):
    def test_prefers_error_over_leading_warning(self) -> None:
        mod = load_measure()
        text = "\n".join(
            [
                "warning[W0007|typecheck]: unused binding `next`",
                "  --> tests/fixtures/x.ark:1:1",
                "error[E0200|typecheck]: type mismatch: expected `i32`, found `str`",
                "  --> tests/fixtures/x.ark:3:5",
            ]
        )
        primary = mod._primary_diagnostic_line(text)
        self.assertIn("error[E0200|typecheck]", primary)
        self.assertNotIn("warning[", primary)

    def test_skips_harness_noise_before_error(self) -> None:
        mod = load_measure()
        text = "\n".join(
            [
                "core-op-shadow: summary candidates=0 matched=0 mismatched=0 unresolved=0 spine=1",
                "reg-vt-audit: summary candidates=1 matched=1 mismatched=0",
                "error[E0401|component]: unsupported world export shape",
            ]
        )
        primary = mod._primary_diagnostic_line(text)
        self.assertIn("error[E0401|component]", primary)

    def test_clang_error_preferred_over_warning(self) -> None:
        mod = load_measure()
        text = "\n".join(
            [
                "/tmp/work/.build/native/program.c:12:3: warning: unused variable 'x'",
                "/tmp/work/.build/native/program.c:40:5: error: incompatible integer to pointer conversion",
            ]
        )
        primary = mod._primary_diagnostic_line(text)
        self.assertIn("error:", primary)
        self.assertNotIn("warning:", primary)


class NormalizeMessageTests(unittest.TestCase):
    def test_normalizes_paths_temp_and_line_col(self) -> None:
        mod = load_measure()
        raw = (
            "/tmp/agent/.build-native-recovery/native/program.c:12:34: error: boom "
            "at 0xdeadbeef"
        )
        normalized = mod._normalize_message(raw)
        self.assertIn("<TEMP>", normalized)
        self.assertIn(":<LINE>:<COL>", normalized)
        self.assertIn("<ADDR>", normalized)
        self.assertNotIn("/tmp/", normalized)
        self.assertNotIn("12:34", normalized)


class FixtureFlagsTests(unittest.TestCase):
    def test_read_fixture_flags_splits_tokens(self) -> None:
        mod = load_measure()
        flags = mod._read_fixture_flags("component/world_command_missing_run.ark")
        self.assertEqual(flags, ["--world", "wasi:cli/command"])

    def test_missing_flags_file_returns_empty(self) -> None:
        mod = load_measure()
        self.assertEqual(mod._read_fixture_flags("component/export_flags.ark"), [])


class DiagnosticsMatchTests(unittest.TestCase):
    def test_order_independent_multiple_required(self) -> None:
        mod = load_measure()
        haystack = "\n".join(
            [
                "warning[W0001|lint]: style",
                "error[E0200|typecheck]: type mismatch",
                "error[E0100|resolve]: undefined name: foo",
            ]
        )
        pattern = "error[E0100|resolve]: undefined name: foo\nerror[E0200|typecheck]: type mismatch"
        self.assertTrue(mod._diagnostics_match(pattern, haystack))

    def test_missing_one_of_multiple_fails(self) -> None:
        mod = load_measure()
        haystack = "error[E0200|typecheck]: type mismatch\n"
        pattern = "error[E0200|typecheck]: type mismatch\nerror[E0100|resolve]: undefined name: foo"
        self.assertFalse(mod._diagnostics_match(pattern, haystack))

    def test_bare_diagnostic_code_matches_bracket_form(self) -> None:
        mod = load_measure()
        haystack = "error[E0401|component]: unsupported export\n"
        self.assertTrue(mod._diagnostics_match("E0401", haystack))

    def test_regexish_needle_matches_numbers(self) -> None:
        mod = load_measure()
        haystack = "error[E0001|parse]: clause has 2 parameters but previous clauses have 1\n"
        pattern = r"clause has \d+ parameters but previous clauses have \d+"
        self.assertTrue(mod._diagnostics_match(pattern, haystack))

    def test_normalized_path_needle_matches(self) -> None:
        mod = load_measure()
        haystack = "/home/user/proj/.build/x/program.c:9:1: error: bad cast\n"
        self.assertTrue(
            mod._diagnostics_match("<TEMP>:<LINE>:<COL>: error: bad cast", haystack)
        )
        self.assertTrue(mod._diagnostics_match("error: bad cast", haystack))


class ExpectationMatchedNegativeTests(unittest.TestCase):
    def test_warning_pattern_matches_even_when_check_exits_zero(self) -> None:
        mod = load_measure()
        matched, reason = mod._expectation_matched(
            expectation={
                "expected_compile": False,
                "expected_run_kind": "not_run",
                "expected_exit_code": None,
                "expected_signal": None,
                "expected_stdout_pattern": None,
                "expected_stderr_pattern": "unused binding `unused_val`",
            },
            compile_kind="compile_pass",
            compile_ok=True,
            combined_compile="warning[W0007|typecheck]: unused binding `unused_val`\n",
            ran=False,
            run_rc=None,
            run_out="",
            run_err="",
            run_signal=None,
        )
        self.assertTrue(matched)
        self.assertEqual(reason, "negative_ok")

    def test_compile_negative_ignores_expected_signal(self) -> None:
        mod = load_measure()
        matched, reason = mod._expectation_matched(
            expectation={
                "expected_compile": False,
                "expected_run_kind": "not_run",
                "expected_exit_code": None,
                "expected_signal": "SIGABRT",
                "expected_stdout_pattern": None,
                "expected_stderr_pattern": "E0401",
            },
            compile_kind="frontend_error",
            compile_ok=False,
            combined_compile="warning[W0007|typecheck]: unused\nerror[E0401|component]: bad\n",
            ran=False,
            run_rc=None,
            run_out="",
            run_err="SIGABRT: abort",
            run_signal="SIGABRT",
        )
        self.assertTrue(matched)
        self.assertEqual(reason, "negative_ok")

    def test_runtime_trap_still_checks_signal_separately(self) -> None:
        mod = load_measure()
        matched, reason = mod._expectation_matched(
            expectation={
                "expected_compile": True,
                "expected_run_kind": "trap",
                "expected_exit_code": None,
                "expected_signal": "SIGABRT",
                "expected_stdout_pattern": None,
                "expected_stderr_pattern": "divide by zero",
            },
            compile_kind="compile_pass",
            compile_ok=True,
            combined_compile="",
            ran=True,
            run_rc=-6,
            run_out="",
            run_err="runtime trap: divide by zero\n",
            run_signal="SIGABRT",
        )
        self.assertTrue(matched)
        self.assertEqual(reason, "abnormal_ok")

    def test_runtime_trap_does_not_use_compile_output_for_stderr(self) -> None:
        mod = load_measure()
        matched, reason = mod._expectation_matched(
            expectation={
                "expected_compile": True,
                "expected_run_kind": "trap",
                "expected_exit_code": None,
                "expected_signal": "SIGABRT",
                "expected_stdout_pattern": None,
                "expected_stderr_pattern": "divide by zero",
            },
            compile_kind="compile_pass",
            compile_ok=True,
            combined_compile="divide by zero",  # must not satisfy runtime diagnostic
            ran=True,
            run_rc=-6,
            run_out="",
            run_err="runtime trap: null reference\n",
            run_signal="SIGABRT",
        )
        self.assertFalse(matched)
        self.assertEqual(reason, "stderr_mismatch")


if __name__ == "__main__":
    unittest.main()
