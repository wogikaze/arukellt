"""wasm32: bare int literals must widen to i64 call params (clock t_resolve)."""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

MULTI_ARG = """
fn make(a: bool, b: i32, c: i32, d: i32, t: i64) -> i64 { t }
fn main() -> i64 { make(true, 1, 2, 3, 0) }
"""

MIXED_NAME = """
fn parse_i64_digits(base: i32, x: i64) -> i64 { x }
fn main() -> i64 { parse_i64_digits(10, 0) }
"""

STRUCT_FIELD = """
struct Stamp { t: i64 }
fn main() -> i64 {
    let s = Stamp { t: 0 }
    s.t
}
"""


class Wasm32I64CallLiteralTests(unittest.TestCase):
    def _compile_validate(self, source: str, stem: str) -> None:
        runtime = ROOT / ".build/selfhost/arukellt-s2-runtime.wasm"
        if not runtime.is_file():
            self.skipTest("s2 runtime wasm is not built")
        wasm_tools = shutil.which("wasm-tools")
        if wasm_tools is None:
            self.skipTest("wasm-tools is required")

        out_dir = ROOT / ".build/tests"
        out_dir.mkdir(parents=True, exist_ok=True)
        src = out_dir / f"{stem}.ark"
        out = out_dir / f"{stem}.wasm"
        src.write_text(source, encoding="utf-8")
        env = os.environ.copy()
        env["ARUKELLT_SELFHOST_WASM"] = str(runtime)
        compile_result = subprocess.run(
            [
                str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                str(src.relative_to(ROOT)),
                "--target",
                "wasm32",
                "--wasi-version",
                "wasi-p1",
                "-o",
                str(out.relative_to(ROOT)),
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
            [wasm_tools, "validate", str(out)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(validate.returncode, 0, validate.stdout + validate.stderr)

    def test_trailing_i64_param_bare_zero_validates(self) -> None:
        self._compile_validate(MULTI_ARG, "i64_call_trailing_lit")

    def test_mixed_i32_i64_params_with_i64_in_name(self) -> None:
        self._compile_validate(MIXED_NAME, "i64_call_mixed_name")

    def test_struct_i64_field_bare_zero_validates(self) -> None:
        self._compile_validate(STRUCT_FIELD, "i64_struct_field_lit")


if __name__ == "__main__":
    unittest.main()
