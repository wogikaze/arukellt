"""wasm32: i32.to_string must emit a valid linear module, not (ref i32)."""
from __future__ import annotations

import os
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = "tests/fixtures/stdlib_io/i32_to_string.ark"


class Wasm32I32ToStringTests(unittest.TestCase):
    def test_wasm32_wasi_p1_compiles_validates_and_runs(self):
        if not (ROOT / ".build/selfhost/arukellt-s2-runtime.wasm").is_file():
            self.skipTest("s2 runtime wasm is not built")
        wasm_tools = shutil.which("wasm-tools")
        if wasm_tools is None:
            self.skipTest("wasm-tools is required")

        runtime = ROOT / ".build/selfhost/arukellt-s2-runtime.wasm"
        output = ROOT / ".build/tests/i32_to_string_wasm32.wasm"
        output.parent.mkdir(parents=True, exist_ok=True)
        env = os.environ.copy()
        env["ARUKELLT_SELFHOST_WASM"] = str(runtime)
        compile_result = subprocess.run(
            [
                str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                FIXTURE,
                "--target",
                "wasm32",
                "--wasi-version",
                "wasi-p1",
                "-o",
                str(output.relative_to(ROOT)),
            ],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(
            compile_result.returncode,
            0,
            compile_result.stdout + compile_result.stderr,
        )
        validate = subprocess.run(
            [wasm_tools, "validate", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(validate.returncode, 0, validate.stdout + validate.stderr)

        run = subprocess.run(
            ["wasmtime", "run", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertEqual(run.stdout.strip(), "0\n42\n-100")


if __name__ == "__main__":
    unittest.main()
