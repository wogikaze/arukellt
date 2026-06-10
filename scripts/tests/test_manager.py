"""Behavioral contract tests for scripts/manager.py."""

import subprocess
import sys
import tempfile
import unittest
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
        result = _run("verify", "quick")
        self.assertIsNotNone(result.returncode)
        self.assertIsInstance(result.returncode, int)
        self.assertIn(result.returncode, (0, 1))

    def test_verify_quick_stdout_has_summary(self):
        result = _run("verify", "quick")
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
