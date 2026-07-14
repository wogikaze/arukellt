"""Behavioral contract tests for scripts/manager.py."""

import subprocess
import sys
import tempfile
import unittest
from unittest import mock
from pathlib import Path

from scripts import manager

REPO_ROOT = Path(__file__).resolve().parent.parent.parent  # ~/arukellt
MANAGER = str(REPO_ROOT / "scripts" / "manager.py")


def _run(*args: str, extra_env: dict | None = None) -> subprocess.CompletedProcess:
    import os

    env = os.environ.copy()
    if extra_env:
        env.update(extra_env)
    return subprocess.run(
        [sys.executable, MANAGER, *args],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        env=env,
    )


class TestVerifyQuick(unittest.TestCase):
    def test_verify_quick_exit_code_type(self):
        result = _run("verify", "quick", "--dry-run")
        self.assertIsNotNone(result.returncode)
        self.assertIsInstance(result.returncode, int)
        self.assertEqual(result.returncode, 0)

    def test_verify_quick_stdout_has_summary(self):
        result = _run("verify", "quick", "--dry-run")
        combined = result.stdout + result.stderr
        self.assertTrue(
            any(
                token in combined
                for token in ("Summary", "checks passed", "checks failed")
            ),
            msg=f"Expected summary output, got:\n{combined[:2000]}",
        )


class TestVerifyDryRun(unittest.TestCase):
    def test_verify_fixtures_dry_run(self):
        result = _run("verify", "fixtures", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)

    def test_verify_size_dry_run(self):
        result = _run("verify", "size", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)

    def test_verify_wat_dry_run(self):
        result = _run("verify", "wat", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)

    def test_verify_component_dry_run(self):
        result = _run("verify", "component", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)

    def test_verify_release_dry_run_includes_full_quality(self):
        result = _run("verify", "release", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertIn("check-code-quality-contract.py", result.stdout)
        self.assertIn("run_fixpoint()", result.stdout)


class TestQualityCommands(unittest.TestCase):
    def test_quality_contract_covers_tracked_file_families(self):
        from contextlib import redirect_stdout
        from io import StringIO

        from scripts.quality.checks import check_quality_contract

        with redirect_stdout(StringIO()):
            self.assertEqual(check_quality_contract(REPO_ROOT), 0)

    def test_quick_cache_fingerprints_dirty_tracked_content(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            tracked = root / "tracked.txt"
            tracked.write_text("index\n", encoding="utf-8")
            subprocess.run(["git", "add", "tracked.txt"], cwd=root, check=True)
            tracked.write_text("worktree\n", encoding="utf-8")

            previous = manager._VERIFY_QUICK_FILE_DIGESTS
            try:
                manager._VERIFY_QUICK_FILE_DIGESTS = None
                manager._prime_quick_file_digest_cache(root)
                digest = manager._VERIFY_QUICK_FILE_DIGESTS["tracked.txt"]
            finally:
                manager._VERIFY_QUICK_FILE_DIGESTS = previous

            self.assertEqual(digest, f"worktree:{manager._file_sha256(tracked)}")

    def test_fmt_check_dry_run_uses_canonical_entrypoint(self):
        result = _run("fmt", "--check", "--dry-run", "src/compiler/fmt/range.ark")
        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertIn("fmt --check", result.stdout + result.stderr)

    def test_lint_dry_run_selects_local_tier_for_package_module(self):
        result = _run("lint", "--dry-run", "src/compiler/fmt/range.ark")
        self.assertEqual(result.returncode, 0, msg=result.stderr)
        output = result.stdout + result.stderr
        self.assertIn("lint --local", output)
        self.assertNotIn("--deny", output)

    def test_lint_can_deny_configured_warning(self):
        from contextlib import redirect_stdout
        from io import StringIO

        from scripts.quality.checks import run_lint

        output = StringIO()
        with redirect_stdout(output):
            result = run_lint(
                REPO_ROOT,
                ["src/compiler/fmt/range.ark"],
                fix=False,
                dry_run=True,
                json_output=False,
                deny_prefer_else_if=True,
            )

        self.assertEqual(result, 0)
        self.assertIn("--deny prefer-else-if", output.getvalue())

    def test_w0011_ratchet_allows_existing_count_and_rejects_increase(self):
        from scripts.quality.checks import run_lint_ratchet

        with mock.patch(
            "scripts.quality.checks._lint_w0011_count",
            return_value=(0, 2, ""),
        ), mock.patch(
            "scripts.quality.checks._base_lint_w0011_count",
            return_value=(2, ""),
        ):
            self.assertEqual(
                run_lint_ratchet(REPO_ROOT, ["same.ark"], "HEAD", False, False),
                0,
            )

        with mock.patch(
            "scripts.quality.checks._lint_w0011_count",
            return_value=(0, 3, ""),
        ), mock.patch(
            "scripts.quality.checks._base_lint_w0011_count",
            return_value=(2, ""),
        ):
            self.assertEqual(
                run_lint_ratchet(REPO_ROOT, ["increased.ark"], "HEAD", False, False),
                1,
            )

    def test_baseline_update_requires_tracking_issue(self):
        result = subprocess.run(
            [
                sys.executable,
                str(REPO_ROOT / "scripts/check/check-ark-code-quality.py"),
                "--write-baseline",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("requires --issue", result.stderr)

    def test_quality_commands_are_registered(self):
        parser = manager.build_parser()
        for command in ("changed", "quick", "structure", "full", "report"):
            args = parser.parse_args(["quality", command, "--dry-run"])
            self.assertEqual(args.domain, "quality")
            self.assertEqual(args.subcommand, command)

    def test_quality_report_declares_advisory_hotspot_purpose(self):
        result = _run("quality", "report")
        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertIn("not a quality score", result.stdout)
        self.assertIn("top hotspots:", result.stdout)


class TestCompilerRefactorGates(unittest.TestCase):
    def test_root_layout_gate_keeps_root_to_facades(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "src" / "compiler"
            compiler.mkdir(parents=True)
            (compiler / "main.ark").write_text("fn main() {}\n", encoding="utf-8")
            (compiler / "parser_expr_atoms.ark").write_text("pub fn parse_atom() {}\n", encoding="utf-8")

            violations = manager._compiler_root_layout_violations(root)

            self.assertEqual(violations, ["src/compiler/parser_expr_atoms.ark"])

    def test_namespace_layout_gate_requires_compiler_subsystems(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "src" / "compiler"
            for rel in (
                "compiler",
                "component",
                "corehir",
                "diagnostics",
                "driver",
                "fmt",
                "hir",
                "lexer",
                "mir",
                "mir/lower",
                "parser",
                "resolver",
                "typechecker",
                "wasm",
                "wasm/intrinsics",
            ):
                directory = compiler / rel
                directory.mkdir(parents=True, exist_ok=True)
                (directory / "mod.ark").write_text("pub fn marker() {}\n", encoding="utf-8")

            violations = manager._compiler_namespace_layout_violations(root)

            self.assertEqual(violations, [])

    def test_constant_function_gate_requires_table_modules(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            parser = root / "src" / "compiler" / "parser"
            parser.mkdir(parents=True)
            (parser / "kinds.ark").write_text(
                "pub fn NK_FN_DECL() -> i32 { 50 }\n",
                encoding="utf-8",
            )
            (root / "src" / "compiler" / "parser_kind_decl.ark").write_text(
                "pub fn NK_STRUCT_DECL() -> i32 { 51 }\n",
                encoding="utf-8",
            )

            violations = manager._compiler_constant_function_layout_violations(root)

            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0][0], "src/compiler/parser_kind_decl.ark")

    def test_fragmented_constant_table_gate_flags_tiny_tables(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "src" / "compiler"
            compiler.mkdir(parents=True)
            (compiler / "mir_type_ops_unary.ark").write_text(
                "pub fn UOP_NEG() -> i32 { 1 }\n"
                "pub fn UOP_NOT() -> i32 { 2 }\n",
                encoding="utf-8",
            )

            violations = manager._compiler_fragmented_constant_table_violations(root)

            self.assertEqual(violations, [("src/compiler/mir_type_ops_unary.ark", 2)])


class TestEnvBehavior(unittest.TestCase):
    def test_env_arukellt_bin_honored(self):
        """ARUKELLT_BIN=/nonexistent with --dry-run must still exit 0."""
        result = _run("verify", "size", "--dry-run", extra_env={"ARUKELLT_BIN": "/nonexistent"})
        self.assertEqual(result.returncode, 0, msg=result.stderr)


class TestUnknownSubcommand(unittest.TestCase):
    def test_unknown_subcommand_exits_nonzero(self):
        result = _run("verify", "bogus")
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
