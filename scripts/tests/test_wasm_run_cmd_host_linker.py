"""cli-parity / compiler run argv must use host-linker for wasi:cli imports."""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from selfhost.checks import _wasm_needs_host_linker, _wasm_run_cmd  # noqa: E402


class WasmRunCmdHostLinkerTests(unittest.TestCase):
    def test_wasi_cli_import_needs_host_linker(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            wasm = Path(tmp) / "compiler.wasm"
            wasm.write_bytes(b"\0asmxxxxwasi:cli/stdout@0.2.0::get-stdout")
            self.assertTrue(_wasm_needs_host_linker(wasm))

    def test_wasm_run_cmd_routes_wasi_cli_through_hosted(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            extra = root / "preopen"
            extra.mkdir()
            wasm = root / "compiler.wasm"
            wasm.write_bytes(b"\0asmxxxxwasi:cli/stdout@0.2.0::get-stdout")
            cmd = _wasm_run_cmd(
                "wasmtime",
                wasm,
                root,
                ["--version"],
                dirs=[root, extra],
            )
            self.assertEqual(cmd[0], "bash")
            self.assertTrue(cmd[1].endswith("arukellt-run-hosted.sh"))
            self.assertIn(f"--dir={root}", cmd)
            self.assertIn(f"--dir={extra}", cmd)
            self.assertNotIn("wasmtime", cmd)
            self.assertEqual(cmd[-3:], [str(wasm), "--", "--version"])

    def test_wasm_run_cmd_plain_wasmtime_without_host_imports(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            wasm = root / "compiler.wasm"
            wasm.write_bytes(b"\0asmxxxxno-bridged-imports")
            cmd = _wasm_run_cmd("wasmtime", wasm, root, ["targets"])
            self.assertEqual(cmd[0], "wasmtime")
            self.assertEqual(cmd[1], "run")
            self.assertIn(str(wasm), cmd)
            self.assertEqual(cmd[-2:], ["--", "targets"])


if __name__ == "__main__":
    unittest.main()
