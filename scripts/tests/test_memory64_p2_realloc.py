"""P2 canonical realloc keeps the memory32 boundary on a Memory64 core."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = "tests/fixtures/ownership/adt_probe.ark"


class P2ReallocMemoryWidthTests(unittest.TestCase):
    def setUp(self) -> None:
        runtime = ROOT / ".build/selfhost/arukellt-s2-runtime.wasm"
        fallback = ROOT / ".build/selfhost/arukellt-s2.wasm"
        if not runtime.is_file() and not fallback.is_file():
            self.skipTest("current-source selfhost wasm is not built")
        if shutil.which("wasm-tools") is None:
            self.skipTest("wasm-tools is required")
        self.compiler = runtime if runtime.is_file() else fallback

    def compile_fixture(self, target: str, output_name: str) -> Path:
        output = ROOT / ".build/tests" / output_name
        output.parent.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [
                str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                FIXTURE,
                "--target",
                target,
                "--emit",
                "wasm",
                "-o",
                str(output.relative_to(ROOT)),
            ],
            cwd=ROOT,
            env={**os.environ, "ARUKELLT_SELFHOST_WASM": str(self.compiler)},
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue(output.is_file())
        validate = subprocess.run(
            ["wasm-tools", "validate", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(validate.returncode, 0, validate.stderr + validate.stdout)
        return output

    def exported_body(self, wat: str, export_name: str) -> str:
        export = re.search(rf'\(export "{export_name}" \(func (\d+)\)\)', wat)
        self.assertIsNotNone(export, f"missing export {export_name}")
        function_index = export.group(1)
        body = re.search(
            rf"\(func \(;{function_index};\)(.*?)(?=\n  \(func|\n\))",
            wat,
            re.S,
        )
        self.assertIsNotNone(body, f"missing body for func {function_index}")
        return body.group(1)

    def test_memory64_p2_realloc_adapts_to_checked_memory32_boundary(self):
        output = self.compile_fixture("wasm32-gc", "p2_realloc_memory64.wasm")
        printed = subprocess.run(
            ["wasm-tools", "print", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(printed.returncode, 0, printed.stderr)
        body = self.exported_body(printed.stdout, "cm32p2_realloc")
        self.assertIn("(local i64 i64)", body)
        self.assertIn("i64.extend_i32_u", body)
        self.assertIn("i64.ge_u", body)
        self.assertIn("i64.gt_u", body)
        self.assertIn("memory.copy", body)
        self.assertIn("i32.wrap_i64", body)

    def test_wasm32_supported_path_remains_memory32(self):
        output = self.compile_fixture("wasm32", "p2_realloc_memory32.wasm")
        printed = subprocess.run(
            ["wasm-tools", "print", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(printed.returncode, 0, printed.stderr)
        self.assertIn("(memory (;0;) 8192)", printed.stdout)
        self.assertNotIn("memory64", printed.stdout)


if __name__ == "__main__":
    unittest.main()
