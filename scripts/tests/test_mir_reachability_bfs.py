# MIR queue-BFS reachability (#823): prune counts via --time; REF_FUNC edge kept.

from __future__ import annotations

import re
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CALL_FIXTURE = "tests/fixtures/reachability/call_export_roots.ark"
REF_FIXTURE_BODY = """\
// Temporary source for REF_FUNC edge keep (#823). Not a T3 validate fixture:
// funcref emit still fails wasm-tools validate (same class as tail_call_ref).

pub fn exported_entry(x: i32) -> i32 {
    via_call(x) + via_ref(x)
}

fn via_call(x: i32) -> i32 {
    x + 1
}

fn via_ref(x: i32) -> i32 {
    let f = via_call
    f(x)
}

fn truly_dead() -> i32 {
    99
}

fn main() {
}
"""


def _compiler_wasm() -> Path | None:
    for rel in (
        ".build/selfhost/arukellt-s2-runtime.wasm",
        ".build/selfhost/arukellt-s2.wasm",
    ):
        path = ROOT / rel
        if path.is_file():
            return path
    return None


def _parse_reachability_fns(text: str) -> tuple[int, int] | None:
    match = re.search(r"lower\.reachability_fns: before=(\d+) after=(\d+)", text)
    if match is None:
        return None
    return int(match.group(1)), int(match.group(2))


class MirReachabilityBfsTests(unittest.TestCase):
    def compile_with_time(self, source_rel: str, out_rel: str) -> str:
        compiler = _compiler_wasm()
        if compiler is None:
            self.skipTest("selfhost compiler wasm is not built")
        wasmtime = shutil.which("wasmtime")
        if wasmtime is None:
            self.skipTest("wasmtime is required")
        out_path = ROOT / out_rel
        out_path.parent.mkdir(parents=True, exist_ok=True)
        if out_path.exists():
            out_path.unlink()
        cmd = [
            wasmtime,
            "run",
            "-W",
            "memory64=y",
            "--dir",
            str(ROOT),
            "--dir",
            str(out_path.parent),
            str(compiler),
            "--",
            "compile",
            source_rel,
            "--target",
            "wasm32-gc",
            "--wasi-version",
            "wasi-p2",
            "--time",
            "-o",
            out_rel,
        ]
        result = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False)
        text = (result.stderr or "") + "\n" + (result.stdout or "")
        self.assertIn("compilation succeeded", text, text)
        return text

    def test_call_export_roots_prunes_dead_keeps_export_chain(self):
        text = self.compile_with_time(
            CALL_FIXTURE,
            ".build/tests/reachability_call_export_roots.wasm",
        )
        counts = _parse_reachability_fns(text)
        self.assertIsNotNone(counts, text)
        before, after = counts
        self.assertGreater(before, after)
        # main + exported_entry + via_call (prelude/stdlib may add more roots).
        self.assertGreaterEqual(after, 3)
        wasm_tools = shutil.which("wasm-tools")
        if wasm_tools is None:
            self.skipTest("wasm-tools is required for validate")
        validate = subprocess.run(
            [
                wasm_tools,
                "validate",
                "--features",
                "gc",
                str(ROOT / ".build/tests/reachability_call_export_roots.wasm"),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(validate.returncode, 0, validate.stdout + validate.stderr)

    def test_ref_func_edge_keeps_via_call_from_export(self):
        src = ROOT / ".build/tests/reachability_ref_func_edge.ark"
        src.parent.mkdir(parents=True, exist_ok=True)
        src.write_text(REF_FIXTURE_BODY, encoding="utf-8")
        text = self.compile_with_time(
            ".build/tests/reachability_ref_func_edge.ark",
            ".build/tests/reachability_ref_func_edge.wasm",
        )
        counts = _parse_reachability_fns(text)
        self.assertIsNotNone(counts, text)
        before, after = counts
        self.assertGreater(before, after)
        # main + exported_entry + via_call + via_ref at minimum.
        self.assertGreaterEqual(after, 4)


if __name__ == "__main__":
    unittest.main()
