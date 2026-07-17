# MIR queue-BFS reachability (#823): CALL prune + REF_FUNC-only keep via MIR dump.

from __future__ import annotations

import os
import re
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CALL_FIXTURE = "tests/fixtures/reachability/call_export_roots.ark"
REF_FIXTURE = "tests/fixtures/reachability/ref_func_only_target.ark"
RECEIPT_PATH = ROOT / ".build" / "selfhost" / "reachability-bfs-receipt.json"


def _compiler_wasm() -> Path | None:
    env = os.environ.get("ARUKELLT_SELFHOST_WASM", "").strip()
    if env:
        path = Path(env)
        if not path.is_absolute():
            path = ROOT / path
        if path.is_file():
            return path
    for rel in (
        ".build/selfhost/arukellt-s2-runtime.wasm",
        ".build/selfhost/arukellt-s2.wasm",
        ".build/selfhost/arukellt-s2-clock.wasm",
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


def _mir_has_fn(mir_text: str, name: str) -> bool:
    return re.search(rf"^\s*fn {re.escape(name)}\b", mir_text, re.M) is not None


def _wasm_tools() -> str | None:
    cargo_tool = Path.home() / ".cargo" / "bin" / "wasm-tools"
    if cargo_tool.is_file():
        return str(cargo_tool)
    return shutil.which("wasm-tools")


class MirReachabilityBfsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.compiler = _compiler_wasm()
        cls.skip_reason = ""
        if cls.compiler is None:
            cls.skip_reason = "selfhost compiler wasm missing (gate must build it first)"
        cls.wasmtime = shutil.which("wasmtime")
        if cls.wasmtime is None and not cls.skip_reason:
            cls.skip_reason = "wasmtime not found in PATH"

    def _require_compiler(self) -> Path:
        if self.skip_reason:
            self.fail(self.skip_reason)
        assert self.compiler is not None
        return self.compiler

    def compile(
        self,
        source_rel: str,
        out_rel: str,
        *,
        dump_mir: bool = False,
        extra_env: dict[str, str] | None = None,
    ) -> tuple[int, str]:
        compiler = self._require_compiler()
        out_path = ROOT / out_rel
        out_path.parent.mkdir(parents=True, exist_ok=True)
        if out_path.exists():
            out_path.unlink()
        args = [
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
        if dump_mir:
            args.extend(["--dump-phases", "mir"])
        cmd = [
            self.wasmtime,
            "run",
            "-W",
            "memory64=y",
            "--dir",
            str(ROOT),
            "--dir",
            str(out_path.parent),
            str(compiler),
            "--",
            *args,
        ]
        env = {**os.environ, **(extra_env or {})}
        result = subprocess.run(
            cmd,
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            env=env,
        )
        text = (result.stderr or "") + "\n" + (result.stdout or "")
        return result.returncode, text

    def test_call_export_roots_prunes_dead_keeps_export_chain(self):
        rc, text = self.compile(
            CALL_FIXTURE,
            ".build/tests/reachability_call_export_roots.wasm",
            dump_mir=True,
        )
        self.assertEqual(rc, 0, text)
        self.assertIn("compilation succeeded", text, text)
        counts = _parse_reachability_fns(text)
        self.assertIsNotNone(counts, text)
        before, after = counts
        self.assertGreater(before, after)
        self.assertTrue(_mir_has_fn(text, "exported_entry"), text)
        self.assertTrue(_mir_has_fn(text, "via_call"), text)
        self.assertFalse(_mir_has_fn(text, "truly_dead"), text)
        wasm_tools = _wasm_tools()
        self.assertIsNotNone(wasm_tools, "wasm-tools is required")
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

    def test_ref_func_only_target_survives_and_truly_dead_is_pruned(self):
        rc, text = self.compile(
            REF_FIXTURE,
            ".build/tests/reachability_ref_func_only.wasm",
            dump_mir=True,
        )
        self.assertEqual(rc, 0, text)
        self.assertIn("compilation succeeded", text, text)
        self.assertTrue(
            _mir_has_fn(text, "ref_only_target"),
            "ref_only_target must remain via REF_FUNC edge:\n" + text,
        )
        self.assertTrue(_mir_has_fn(text, "via_ref"), text)
        self.assertTrue(_mir_has_fn(text, "exported_entry"), text)
        self.assertFalse(
            _mir_has_fn(text, "truly_dead"),
            "truly_dead must be pruned:\n" + text,
        )


if __name__ == "__main__":
    unittest.main()
