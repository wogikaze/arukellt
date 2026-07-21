"""Vec<i32> get_unchecked in a Vec<Struct>-returning fn must validate under wasm32-gc.

Regression for #730: GC local inference used to type the i32 element as the
enclosing function's struct return (vec:ModInfo → ModInfo), so Memory64
i64.extend + local.set failed validation.
"""

from pathlib import Path
import shutil
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = "tests/fixtures/memory64_vec_i32_in_struct_fn/build_module_infos.ark"


class Memory64VecI32InStructFnTests(unittest.TestCase):
    def test_build_module_infos_shape_validates_under_wasm32_gc(self):
        if not (ROOT / ".build/selfhost/arukellt-s2-runtime.wasm").is_file() and not (
            ROOT / ".build/selfhost/arukellt-s2.wasm"
        ).is_file():
            self.skipTest("current-source selfhost wasm is not built")
        wasm_tools = shutil.which("wasm-tools")
        if wasm_tools is None:
            self.skipTest("wasm-tools is required")

        output = ROOT / ".build/tests/memory64_vec_i32_in_struct_fn.wasm"
        output.parent.mkdir(parents=True, exist_ok=True)
        result = subprocess.run(
            [
                str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                FIXTURE,
                "--target",
                "wasm32-gc",
                "--wasi-version",
                "wasi-p2",
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
            [wasm_tools, "validate", "--features", "gc,memory64", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(validate.returncode, 0, validate.stderr + validate.stdout)


if __name__ == "__main__":
    unittest.main()
