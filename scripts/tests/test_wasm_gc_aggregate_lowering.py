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
)
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
        self.compiler = compiler

    def compile_fixture(self, fixture: str, target: str) -> Path:
        stem = Path(fixture).stem
        output = ROOT / ".build/tests" / f"aggregate-{target}-{stem}.wasm"
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
        command.extend(("--emit", "wasm", "-o", str(output.relative_to(ROOT))))
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

    def test_gc_primary_uses_gc_aggregate_ops_and_linear_path_does_not(self) -> None:
        for fixture in FIXTURES:
            with self.subTest(fixture=fixture):
                gc_wat = self.print_wat(self.compile_fixture(fixture, "wasm32-gc"))
                linear_wat = self.print_wat(self.compile_fixture(fixture, "wasm32"))
                self.assertTrue(
                    any(prefix in gc_wat for prefix in GC_AGGREGATE_INSTRUCTION_PREFIXES),
                    f"GC primary silently fell back for {fixture}",
                )
                self.assertNotIn("struct.", linear_wat)
                self.assertNotIn("array.", linear_wat)
                self.assertNotIn("ref.", linear_wat)


if __name__ == "__main__":
    unittest.main()
