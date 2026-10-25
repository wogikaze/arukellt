"""Regression tests for the compat-orphan `deprecated_by` target audit.

The checker must accept a `Type::method` replacement only when an `impl`
block under std actually declares that method. Before #703 Wave 0 any
qualified target that was not a `std::` module path was reported as an
orphan, which blocked method-level deprecation metadata.
"""
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

_SCRIPT = (
    Path(__file__).resolve().parents[1] / "check" / "check-compat-orphans.py"
)
_SPEC = importlib.util.spec_from_file_location("compat_orphans", _SCRIPT)
assert _SPEC and _SPEC.loader
compat_orphans = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = compat_orphans
_SPEC.loader.exec_module(compat_orphans)


class MethodTargetTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "std/collections").mkdir(parents=True)
        (self.root / "std/text").mkdir(parents=True)
        (self.root / "std/collections/vec.ark").write_text(
            "impl Vec<T> {\n"
            "    fn new() -> Vec<T> {}\n"
            "    fn sort(self) {}\n"
            "}\n",
            encoding="utf-8",
        )
        (self.root / "std/text/mod.ark").write_text(
            "pub fn concat(a: String, b: String) -> String {}\n",
            encoding="utf-8",
        )
        self.old_root = compat_orphans.ROOT
        compat_orphans.ROOT = self.root

    def tearDown(self) -> None:
        compat_orphans.ROOT = self.old_root
        self.temp.cleanup()

    def test_method_decl_exists_reads_impl_blocks(self) -> None:
        self.assertTrue(compat_orphans.method_decl_exists(self.root, "Vec", "sort"))
        self.assertTrue(compat_orphans.method_decl_exists(self.root, "Vec<T>", "new"))
        self.assertFalse(compat_orphans.method_decl_exists(self.root, "Vec", "missing"))
        self.assertFalse(compat_orphans.method_decl_exists(self.root, "HashMap", "insert"))

    def test_type_method_target_resolves(self) -> None:
        self.assertTrue(
            compat_orphans.resolve_deprecated_target("Vec::sort", set(), set())
        )
        self.assertFalse(
            compat_orphans.resolve_deprecated_target("Vec::missing", set(), set())
        )

    def test_std_module_and_bare_targets_still_resolve(self) -> None:
        self.assertTrue(
            compat_orphans.resolve_deprecated_target("std::text::concat", set(), set())
        )
        self.assertFalse(
            compat_orphans.resolve_deprecated_target("std::text::missing", set(), set())
        )
        self.assertTrue(
            compat_orphans.resolve_deprecated_target("byte_at", {"byte_at"}, set())
        )
        self.assertFalse(
            compat_orphans.resolve_deprecated_target("byte_at", set(), set())
        )


if __name__ == "__main__":
    unittest.main()
