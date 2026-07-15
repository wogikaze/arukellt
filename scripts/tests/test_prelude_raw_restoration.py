# Sealed raw API path helpers (RFC-006 / #817) — no compiler rebuild required.

from pathlib import Path
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
        self.assertIn("raw_string_clone", text)

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


if __name__ == "__main__":
    unittest.main()
