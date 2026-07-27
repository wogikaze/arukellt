"""Memory64 GC struct i32 fields must emit i32.wrap_i64 before struct.set."""

from pathlib import Path
import re
import shutil
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = "tests/fixtures/memory64_struct_i32/default_fields.ark"


class Memory64StructI32WrapTests(unittest.TestCase):
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

    def test_opts_default_wraps_i32_fields_under_wasm32_gc(self):
        if not (ROOT / ".build/selfhost/arukellt-s2-runtime.wasm").is_file() and not (
            ROOT / ".build/selfhost/arukellt-s2.wasm"
        ).is_file():
            self.skipTest("current-source selfhost wasm is not built")
        wasm_tools = shutil.which("wasm-tools")
        if wasm_tools is None:
            self.skipTest("wasm-tools is required")

        output = ROOT / ".build/tests/memory64_struct_i32_default_fields.wasm"
        output.parent.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [
                str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                FIXTURE,
                "--target",
                "wasm32-gc",
                "-o",
                str(output.relative_to(ROOT)),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue(output.is_file())

        validate = subprocess.run(
            [wasm_tools, "validate", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(validate.returncode, 0, validate.stderr + validate.stdout)

        printed = subprocess.run(
            [wasm_tools, "print", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(printed.returncode, 0, printed.stderr)
        body = self.exported_body(printed.stdout, "Opts_default")
        # Two i32 fields → two wrap+set pairs (name/ref field has no wrap).
        wraps = len(re.findall(r"i32\.wrap_i64\s*\n\s*struct\.set", body))
        self.assertGreaterEqual(wraps, 2, body)


if __name__ == "__main__":
    unittest.main()
