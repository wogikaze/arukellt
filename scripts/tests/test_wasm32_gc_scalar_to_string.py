"""wasm32-gc: bool/f64 Display::to_string must not be drop+unreachable."""
from __future__ import annotations

import os
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HOSTED = ROOT / "scripts/run/arukellt-run-hosted.sh"
PROBE = """use std::host::stdio

fn main() {
    stdio::println((true).to_string())
    stdio::println((false).to_string())
    stdio::println((3.14).to_string())
}
"""


class Wasm32GcScalarToStringTests(unittest.TestCase):
    def test_bool_and_f64_to_string_run(self):
        wasm_tools = shutil.which("wasm-tools")
        if wasm_tools is None:
            self.skipTest("wasm-tools is required")
        runtime_env = os.environ.get("ARUKELLT_SELFHOST_WASM", "")
        runtime = Path(runtime_env) if runtime_env else ROOT / ".build/selfhost/arukellt-s2-runtime.wasm"
        if not runtime.is_file():
            self.skipTest("s2 runtime wasm is not built")
        probe_dir = ROOT / ".build/tests"
        probe_dir.mkdir(parents=True, exist_ok=True)
        source = probe_dir / "scalar_to_string_gc_probe.ark"
        source.write_text(PROBE, encoding="utf-8")
        output = probe_dir / "scalar_to_string_gc_probe.wasm"
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
            ["bash", str(HOSTED), f"--dir={ROOT}", str(output)],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        lines = [line for line in run.stdout.splitlines() if line.strip()]
        self.assertGreaterEqual(len(lines), 3, run.stdout + run.stderr)
        self.assertEqual(lines[0], "true")
        self.assertEqual(lines[1], "false")
        self.assertEqual(lines[2], "3.14")


if __name__ == "__main__":
    unittest.main()
