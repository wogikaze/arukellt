"""Behavioral contract tests for scripts.check.check-t3-wasm-validate."""

import importlib.util
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "check" / "check-t3-wasm-validate.py"


def _load_check_module():
    spec = importlib.util.spec_from_file_location("check_t3_wasm_validate", SCRIPT)
    assert spec is not None and spec.loader is not None
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


class TestFindSelfhostWasm(unittest.TestCase):
    def test_prefers_s2_runtime_over_s3_and_s2(self):
        mod = _load_check_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / ".build" / "selfhost").mkdir(parents=True)
            (root / ".build" / "selfhost" / "arukellt-s3.wasm").write_text("s3")
            (root / ".build" / "selfhost" / "arukellt-s2.wasm").write_text("s2")
            (root / ".build" / "selfhost" / "arukellt-s2-runtime.wasm").write_text("runtime")
            result = mod.find_selfhost_wasm(root)
            self.assertEqual(result, root / ".build" / "selfhost" / "arukellt-s2-runtime.wasm")

    def test_env_var_overrides_files(self):
        mod = _load_check_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / ".build" / "selfhost").mkdir(parents=True)
            runtime = root / ".build" / "selfhost" / "arukellt-s2-runtime.wasm"
            runtime.write_text("runtime")
            env_path = root / "custom.wasm"
            env_path.write_text("custom")
            old_env = os.environ.get("ARUKELLT_SELFHOST_WASM")
            os.environ["ARUKELLT_SELFHOST_WASM"] = str(env_path)
            try:
                result = mod.find_selfhost_wasm(root)
                self.assertEqual(result, env_path)
            finally:
                if old_env is None:
                    os.environ.pop("ARUKELLT_SELFHOST_WASM", None)
                else:
                    os.environ["ARUKELLT_SELFHOST_WASM"] = old_env


class TestCompileFixtureWasmtimeArgs(unittest.TestCase):
    def test_passes_memory64_gc_and_function_references(self):
        mod = _load_check_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "compiler.wasm"
            compiler.write_text("wasm")
            out = root / "out.wasm"

            calls = []

            def fake_run(args, **kwargs):
                calls.append((list(args), kwargs))
                # create the file compile_fixture looks for
                produced = root / ".ark-t3-validate-tmp" / "out.wasm"
                produced.parent.mkdir(parents=True, exist_ok=True)
                produced.write_text("ok")
                return MagicMock(returncode=0, stderr="")

            old_run = mod.subprocess.run
            mod.subprocess.run = fake_run
            try:
                ok, _ = mod.compile_fixture(
                    "wasmtime",
                    compiler,
                    "tests/fixtures/example.ark",
                    out,
                    root,
                )
                self.assertTrue(ok)
            finally:
                mod.subprocess.run = old_run

            self.assertEqual(len(calls), 1)
            args, kwargs = calls[0]
            self.assertEqual(args[0], "wasmtime")
            self.assertEqual(args[1], "run")
            self.assertIn("--wasm", args)
            self.assertIn("gc", args)
            self.assertIn("function-references", args)
            self.assertIn("-W", args)
            self.assertIn("memory64=y", args)
            # memory64=y follows -W
            idx_w = args.index("-W")
            self.assertEqual(args[idx_w + 1], "memory64=y")
            self.assertIn("--dir", args)
            idx_dir = args.index("--dir")
            self.assertEqual(args[idx_dir + 1], str(root))
            self.assertEqual(kwargs.get("cwd"), str(root))


if __name__ == "__main__":
    unittest.main()
