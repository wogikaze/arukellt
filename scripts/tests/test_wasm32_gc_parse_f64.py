"""wasm32-gc: parse_f64 must emit the intrinsic, not drop+unreachable."""
from __future__ import annotations

import os
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HOSTED = ROOT / "scripts/run/arukellt-run-hosted.sh"
PROBE = """use std::test

fn main() {
    match parse_f64("42") {
        Result::Ok(n) => test::assert_true(n > 41.0 && n < 43.0),
        Result::Err(_) => panic("parse_f64 42 failed"),
    }
    match parse_f64("3.14") {
        Result::Ok(n) => test::assert_eq_f64(n, 3.14),
        Result::Err(_) => panic("parse_f64 3.14 failed"),
    }
}
"""


class Wasm32GcParseF64Tests(unittest.TestCase):
    def test_parse_f64_compiles_validates_and_runs(self):
        wasm_tools = shutil.which("wasm-tools")
        if wasm_tools is None:
            self.skipTest("wasm-tools is required")
        runtime_env = os.environ.get("ARUKELLT_SELFHOST_WASM", "")
        if runtime_env:
            runtime = Path(runtime_env)
        else:
            runtime = ROOT / ".build/selfhost/arukellt-s2-runtime.wasm"
        if not runtime.is_file():
            self.skipTest("s2 runtime wasm is not built")
        probe_dir = ROOT / ".build/tests"
        probe_dir.mkdir(parents=True, exist_ok=True)
        source = probe_dir / "parse_f64_gc_probe.ark"
        source.write_text(PROBE, encoding="utf-8")
        output = probe_dir / "parse_f64_gc_probe.wasm"
        env = os.environ.copy()
        env["ARUKELLT_SELFHOST_WASM"] = str(runtime)
        compile_result = subprocess.run(
            [
                str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                str(source.relative_to(ROOT)),
                "--target",
                "wasm32-gc",
                "-o",
                str(output.relative_to(ROOT)),
            ],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        combined = compile_result.stdout + compile_result.stderr
        self.assertEqual(compile_result.returncode, 0, combined)
        self.assertIn("name=0", combined)
        self.assertIn("fallback=0", combined)
        self.assertIn("unresolved=0", combined)
        validate = subprocess.run(
            [
                wasm_tools,
                "validate",
                "--features=gc,function-references,tail-call",
                str(output),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(validate.returncode, 0, validate.stdout + validate.stderr)
        if not HOSTED.is_file():
            self.skipTest("hosted runner is required")
        run = subprocess.run(
            [
                "bash",
                str(HOSTED),
                f"--dir={ROOT}",
                str(output),
            ],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)


if __name__ == "__main__":
    unittest.main()
