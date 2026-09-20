"""Prove aggregate lowering stays on the selected Wasm representation path."""

from __future__ import annotations

import os
import shutil
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = (
    "tests/fixtures/structs/vec_in_struct_enum_field.ark",
    "tests/fixtures/collections/vec_option_i32.ark",
    "tests/fixtures/collections/vec_result_i32.ark",
    "tests/fixtures/variables/tuple.ark",
    "tests/fixtures/enums/enum_struct_variant.ark",
    "tests/fixtures/enums/exhaustive_match.ark",
    "tests/fixtures/enums/nested_enum.ark",
    "tests/fixtures/enums/option_none.ark",
    "tests/fixtures/enums/option_some.ark",
    "tests/fixtures/enums/result_err.ark",
    "tests/fixtures/enums/result_ok.ark",
    "tests/fixtures/enums/tuple_variant.ark",
    "tests/fixtures/enums/unit_variant.ark",
)
ENUM_FIXTURES = FIXTURES[4:]
GC_AGGREGATE_INSTRUCTION_PREFIXES = (
    "struct.new",
    "struct.get",
    "struct.set",
    "array.new",
    "array.get",
    "array.set",
)


class WasmGcAggregateLoweringTest(unittest.TestCase):
    def setUp(self) -> None:
        compiler_candidates = (
            ROOT / ".build/selfhost/arukellt-s2-runtime.wasm",
            ROOT / ".build/selfhost/arukellt-s2.wasm",
        )
        compiler = next((path for path in compiler_candidates if path.is_file()), None)
        if compiler is None:
            self.skipTest("current-source selfhost wasm is not built")
        if shutil.which("wasm-tools") is None:
            self.skipTest("wasm-tools is required")
        if shutil.which("wasmtime") is None:
            self.skipTest("wasmtime is required")
        self.compiler = compiler

    def compile_fixture(self, fixture: str, target: str) -> Path:
        stem = Path(fixture).stem
        artifact_kind = "component" if target == "wasm32-gc" else "wasm"
        output = ROOT / ".build/tests" / f"aggregate-{target}-{stem}.{artifact_kind}.wasm"
        output.parent.mkdir(parents=True, exist_ok=True)
        command = [
            str(ROOT / "scripts/run/arukellt-selfhost.sh"),
            "compile",
            fixture,
            "--target",
            target,
            "--wasi-version",
            "wasi-p2" if target == "wasm32-gc" else "wasi-p1",
        ]
        if target == "wasm32":
            command.extend(("--memory-width", "memory32"))
        command.extend(("--emit", artifact_kind, "-o", str(output.relative_to(ROOT))))
        result = subprocess.run(
            command,
            cwd=ROOT,
            env={**os.environ, "ARUKELLT_SELFHOST_WASM": str(self.compiler)},
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue(output.is_file())
        validated = subprocess.run(
            ["wasm-tools", "validate", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            timeout=30,
        )
        self.assertEqual(validated.returncode, 0, validated.stderr)
        return output

    def print_wat(self, artifact: Path) -> str:
        printed = subprocess.run(
            ["wasm-tools", "print", str(artifact)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            timeout=30,
        )
        self.assertEqual(printed.returncode, 0, printed.stderr)
        return printed.stdout

    def run_fixture(self, fixture: str, target: str, artifact: Path) -> None:
        command = ["wasmtime", "run"]
        if target == "wasm32-gc":
            command.extend(("--wasm", "gc", "--wasm", "function-references"))
        command.extend(("--dir=.", str(artifact)))
        executed = subprocess.run(
            command,
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            timeout=60,
        )
        expected = (ROOT / fixture).with_suffix(".expected").read_text(encoding="utf-8")
        self.assertEqual(executed.returncode, 0, executed.stderr)
        self.assertEqual(executed.stdout, expected)

    def compile_failure(self, fixture: str, target: str) -> str:
        command = [
            str(ROOT / "scripts/run/arukellt-selfhost.sh"),
            "compile",
            fixture,
            "--target",
            target,
            "--wasi-version",
            "wasi-p2" if target == "wasm32-gc" else "wasi-p1",
            "--emit",
            "component" if target == "wasm32-gc" else "wasm",
        ]
        if target == "wasm32":
            command.extend(("--memory-width", "memory32"))
        result = subprocess.run(
            command,
            cwd=ROOT,
            env={**os.environ, "ARUKELLT_SELFHOST_WASM": str(self.compiler)},
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
        )
        self.assertNotEqual(result.returncode, 0, result.stdout)
        return result.stdout + result.stderr

    def test_gc_primary_uses_gc_aggregate_ops_and_linear_path_does_not(self) -> None:
        for fixture in FIXTURES:
            with self.subTest(fixture=fixture):
                gc_artifact = self.compile_fixture(fixture, "wasm32-gc")
                linear_artifact = self.compile_fixture(fixture, "wasm32")
                gc_wat = self.print_wat(gc_artifact)
                linear_wat = self.print_wat(linear_artifact)
                self.assertTrue(
                    any(prefix in gc_wat for prefix in GC_AGGREGATE_INSTRUCTION_PREFIXES),
                    f"GC primary silently fell back for {fixture}",
                )
                if fixture in ENUM_FIXTURES:
                    self.assertIn("struct.new", gc_wat)
                    self.assertIn("struct.get", gc_wat)
                    self.assertIn("ref.cast", gc_wat)
                self.assertNotIn("struct.", linear_wat)
                self.assertNotIn("array.", linear_wat)
                self.assertNotIn("ref.", linear_wat)
                self.run_fixture(fixture, "wasm32-gc", gc_artifact)
                self.run_fixture(fixture, "wasm32", linear_artifact)

    def test_invalid_aggregate_match_is_rejected_before_backend_emit(self) -> None:
        diagnostic = self.compile_failure(
            "tests/fixtures/selfhost/match_non_exhaustive_enum.ark",
            "wasm32-gc",
        )
        self.assertIn("E0200", diagnostic)
        self.assertIn("missing Direction::West", diagnostic)


if __name__ == "__main__":
    unittest.main()
