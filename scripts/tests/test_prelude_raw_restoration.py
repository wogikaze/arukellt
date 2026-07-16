# Sealed raw API path helpers (RFC-006 / #817) — no compiler rebuild required.

from pathlib import Path
import shutil
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class SealedRawApiPathTests(unittest.TestCase):
    def test_module_sealed_raw_source_exists(self):
        path = ROOT / "src/compiler/loader/module_sealed_raw.ark"
        self.assertTrue(path.is_file())
        text = path.read_text(encoding="utf-8")
        self.assertIn('String_from("std::core::raw")', text)
        self.assertIn('String_from("core::raw")', text)
        self.assertIn("sealed stdlib raw API", text)

    def test_raw_stdlib_module_exists(self):
        path = ROOT / "std/core/raw.ark"
        self.assertTrue(path.is_file())
        text = path.read_text(encoding="utf-8")
        self.assertIn("raw_array_new", text)
        self.assertIn("Vec_new_i32_with_cap(cap)", text)
        self.assertIn("raw_string_from_bytes", text)
        self.assertIn("raw_string_clone", text)

    def test_user_import_paths_are_rejected(self):
        for name, import_path in (
            ("user_import_core", "core::raw"),
            ("user_import_std", "std::core::raw"),
        ):
            source_path = ROOT / ".build/tests" / f"{name}.ark"
            source_path.parent.mkdir(parents=True, exist_ok=True)
            source_path.write_text(
                f"use {import_path}\n\nfn main() {{\n    let _value = raw::raw_string_new()\n}}\n",
                encoding="utf-8",
            )
            result = subprocess.run(
                [
                    str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                    "check",
                    str(source_path.relative_to(ROOT)),
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("sealed stdlib raw API", result.stdout + result.stderr)

    def test_raw_storage_differential_fixture_is_registered(self):
        manifest = (ROOT / "tests/fixtures/manifest.txt").read_text(encoding="utf-8")
        fixture = "sealed_raw/raw_storage_differential.ark"
        self.assertIn(f"run:{fixture}", manifest)
        self.assertIn(f"t3-run:{fixture}", manifest)

    def test_vec_uses_sealed_raw(self):
        text = (ROOT / "std/collections/vec.ark").read_text(encoding="utf-8")
        self.assertIn("use std::core::raw", text)
        self.assertIn("raw::raw_array_new", text)

    def test_no_lm_dual_files_remain(self):
        wasm = ROOT / "src/compiler/wasm"
        lm = list(wasm.glob("intrinsic_*_lm.ark"))
        self.assertEqual(lm, [], f"unexpected LM dual files: {lm}")

    def test_module_graph_threads_stdlib_importer_flag(self):
        text = (ROOT / "src/compiler/loader/module_graph.ark").read_text(encoding="utf-8")
        self.assertIn("importer_is_stdlib", text)
        self.assertIn("module_sealed_raw", text)


class PreludeRestorationSourceTests(unittest.TestCase):
    def test_skip_prelude_removed(self):
        decls = (ROOT / "src/compiler/loader/module_decls.ark").read_text(encoding="utf-8")
        self.assertNotIn("combine_loaded_and_main_decls_skip_prelude", decls)
        pipeline = (ROOT / "src/compiler/driver/pipeline_backend.ark").read_text(encoding="utf-8")
        self.assertNotIn("skip_prelude", pipeline)
        self.assertIn("combine_loaded_and_main_decls", pipeline)

    def test_stdlib_body_defer_exists(self):
        path = ROOT / "src/compiler/mir/lower/stdlib_body_defer.ark"
        self.assertTrue(path.is_file())
        text = path.read_text(encoding="utf-8")
        self.assertIn("mir_should_defer_stdlib_body", text)

    def test_current_selfhost_compiles_prelude_and_user_body(self):
        current_selfhost = ROOT / ".build/selfhost/arukellt-s2.wasm"
        if not current_selfhost.is_file():
            self.skipTest("current-source selfhost wasm is not built")
        wasm_tools = shutil.which("wasm-tools")
        if wasm_tools is None:
            self.skipTest("wasm-tools is not installed")

        output = ROOT / ".build/tests/prelude_compiled_user_body.wasm"
        output.parent.mkdir(parents=True, exist_ok=True)
        compile_result = subprocess.run(
            [
                str(ROOT / "scripts/run/arukellt-selfhost.sh"),
                "compile",
                "tests/fixtures/prelude_compiled_user_body/main.ark",
                "--target",
                "wasm32",
                "-o",
                ".build/tests/prelude_compiled_user_body.wasm",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(compile_result.returncode, 0, compile_result.stdout + compile_result.stderr)
        self.assertTrue(output.is_file())

        names = subprocess.run(
            [wasm_tools, "print", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(names.returncode, 0, names.stdout + names.stderr)
        self.assertIn('(export "user_add_seven"', names.stdout)
        self.assertIn('(export "user_search"', names.stdout)
        self.assertIn('(export "user_sequence_checks"', names.stdout)
        self.assertNotIn('(export "sort_by"', names.stdout)

        validate = subprocess.run(
            [wasm_tools, "validate", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(validate.returncode, 0, validate.stdout + validate.stderr)


if __name__ == "__main__":
    unittest.main()
