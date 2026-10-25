"""Regression tests for ADR-046 stdlib eradication classification.

Covers the source-surface extension added by #703 Wave 0: source declaration
enumeration, catch-all errors, intrinsic-only strictness, and manifest/decl
drift. The repo gate is also exercised as an integration check.
"""
from __future__ import annotations

import contextlib
import importlib.util
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

_SCRIPTS = Path(__file__).resolve().parents[1]
if str(_SCRIPTS / "gen") not in sys.path:
    sys.path.insert(0, str(_SCRIPTS / "gen"))

import stdlib_eradication as se  # noqa: E402

_REPO_ROOT = _SCRIPTS.parent
_CHECK_SCRIPT = _SCRIPTS / "check" / "check-stdlib-eradication.py"
_CHECK_SPEC = importlib.util.spec_from_file_location(
    "check_stdlib_eradication", _CHECK_SCRIPT
)
assert _CHECK_SPEC and _CHECK_SPEC.loader
check_stdlib_eradication = importlib.util.module_from_spec(_CHECK_SPEC)
sys.modules[_CHECK_SPEC.name] = check_stdlib_eradication
_CHECK_SPEC.loader.exec_module(check_stdlib_eradication)


class SourceDeclarationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "std").mkdir()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _write(self, relative: str, text: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def test_module_for_path_handles_mod_and_prelude(self) -> None:
        self.assertEqual(
            se.module_for_path(self.root, self.root / "std/collections/hash_map.ark"),
            "std::collections::hash_map",
        )
        self.assertEqual(
            se.module_for_path(self.root, self.root / "std/text/mod.ark"),
            "std::text",
        )
        self.assertIsNone(
            se.module_for_path(self.root, self.root / "std/prelude.ark")
        )

    def test_only_top_level_pub_fns_are_declarations(self) -> None:
        self._write(
            "std/collections/demo.ark",
            "pub fn free_one() {}\n"
            "pub fn generic<T: Ord>(v: Vec<T>) {}\n"
            "impl Vec<T> {\n"
            "    fn method(self) {}\n"
            "}\n"
            "fn private_one() {}\n",
        )
        declarations = se.iter_source_declarations(self.root)
        self.assertEqual([entry["name"] for entry in declarations], ["free_one", "generic"])
        self.assertEqual(declarations[0]["path"], "std/collections/demo.ark")
        self.assertEqual(declarations[0]["line"], 1)

    def test_manifest_entries_without_decl_scopes_compiler_kinds(self) -> None:
        manifest = {
            "functions": [
                {"name": "wrapper", "kind": "prelude_wrapper"},
                {"name": "module_fn", "module": "std::demo"},
                {"name": "builtin_fn", "kind": "builtin"},
                {"name": "intrinsic_fn", "kind": "intrinsic"},
                {"name": "__intrinsic_hidden"},
                {"name": "declared", "module": "std::demo"},
            ]
        }
        missing = se.manifest_entries_without_decl(manifest, {"declared"})
        self.assertEqual(
            [entry["name"] for entry in missing],
            ["wrapper", "module_fn"],
        )

    def test_classify_source_reports_catch_all_as_unregistered(self) -> None:
        self._write(
            "std/demo.ark",
            "pub fn covered() {}\n"
            "pub fn hashset_thing() {}\n"
            "pub fn mystery() {}\n",
        )
        declarations = se.iter_source_declarations(self.root)
        eradication = {
            "rules": [
                {
                    "label": "hash set adapters",
                    "source_only": True,
                    "prefix": "hashset_",
                    "class": "associated-or-method",
                    "owner": "917",
                },
                {
                    "label": "covered",
                    "source_only": True,
                    "name": "covered",
                    "class": "must delete",
                    "owner": "917",
                },
            ],
            "source_default": {
                "label": "unclassified",
                "class": se.SOURCE_ERROR_CLASS,
                "owner": "917",
            },
        }
        resolved, unregistered, errors = se.classify_source(
            eradication, {"functions": []}, declarations
        )
        self.assertEqual([entry["name"] for entry, _rule in resolved], ["covered", "hashset_thing"])
        self.assertEqual(len(unregistered), 3)
        self.assertEqual(len(errors), 1)
        self.assertIn("unregistered std free: std/demo.ark:3 mystery", errors[0])

    def test_registered_names_are_not_double_counted_as_source_only(self) -> None:
        self._write("std/prelude.ark", "pub fn known() {}\n")
        declarations = se.iter_source_declarations(self.root)
        resolved, unregistered, errors = se.classify_source(
            {"rules": [], "source_default": None},
            {"functions": [{"name": "known"}]},
            declarations,
        )
        self.assertEqual((resolved, unregistered, errors), ([], [], []))

    def test_source_only_rule_does_not_match_manifest_entries(self) -> None:
        rule = {
            "label": "source only",
            "source_only": True,
            "module": "std::demo",
            "class": "must delete",
        }
        self.assertFalse(
            se.rule_matches(rule, {"name": "x", "module": "std::demo"})
        )
        self.assertTrue(
            se.rule_matches(
                rule, {"name": "x", "module": "std::demo", "source": True}
            )
        )

    def test_intrinsic_only_rejects_public_wrapper_kind(self) -> None:
        self.assertTrue(
            se.intrinsic_is_non_public(
                {"name": "__intrinsic_memory_copy", "kind": "intrinsic_wrapper"}
            )
        )
        self.assertTrue(
            se.intrinsic_is_non_public({"name": "hidden", "kind": "intrinsic"})
        )
        self.assertFalse(
            se.intrinsic_is_non_public(
                {"name": "memory_copy", "kind": "intrinsic_wrapper"}
            )
        )


class GateErrorClassTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "std").mkdir()
        (self.root / "docs/data").mkdir(parents=True)
        (self.root / "issues/open").mkdir(parents=True)
        (self.root / "issues/open/917-test.md").write_text("# 917\n", encoding="utf-8")
        self.old_root = check_stdlib_eradication.ROOT
        check_stdlib_eradication.ROOT = self.root

    def tearDown(self) -> None:
        check_stdlib_eradication.ROOT = self.old_root
        self.temp.cleanup()

    def _write_manifest(self, text: str) -> None:
        (self.root / "std/manifest.toml").write_text(text, encoding="utf-8")

    def _write_eradication(self, text: str) -> None:
        (self.root / "docs/data/stdlib-api-eradication.toml").write_text(
            text, encoding="utf-8"
        )

    def _run(self) -> tuple[int, str]:
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            rc = check_stdlib_eradication.main()
        return rc, stderr.getvalue()

    def test_reports_unregistered_free_and_manifest_entry_without_decl(self) -> None:
        self._write_manifest(
            '[[functions]]\nname = "ghost_free"\nmodule = "std::demo"\n'
            'stability = "experimental"\nparams = []\nreturns = "()"\n'
            'doc_category = "demo"\n'
        )
        self._write_eradication(
            '[[rules]]\nlabel = "all"\nmanifest_only = true\n'
            'class = "associated-or-method"\n'
            'owner = "917"\nreplacement = "method"\n'
            '[source_default]\nlabel = "unclassified"\nclass = "unregistered"\n'
            'owner = "917"\n'
        )
        (self.root / "std/demo.ark").write_text(
            "pub fn other_free() {}\n", encoding="utf-8"
        )
        rc, err = self._run()
        self.assertEqual(rc, 1)
        self.assertIn("manifest entry without decl: std::demo::ghost_free", err)
        self.assertIn("unregistered std free: std/demo.ark:1 other_free", err)

    def test_rejects_public_intrinsic_wrapper_as_intrinsic_only(self) -> None:
        self._write_manifest(
            '[[functions]]\nname = "memory_copy"\nmodule = "std::wasm"\n'
            'kind = "intrinsic_wrapper"\nintrinsic = "__intrinsic_memory_copy"\n'
            'stability = "experimental"\nparams = ["i32", "i32", "i32"]\n'
            'returns = "()"\ndoc_category = "wasm"\n'
        )
        (self.root / "std/wasm").mkdir()
        (self.root / "std/wasm/mod.ark").write_text(
            "pub fn memory_copy(dst: i32, src: i32, len: i32) {}\n",
            encoding="utf-8",
        )
        self._write_eradication(
            '[[rules]]\nlabel = "wasm frees"\nmodule = "std::wasm"\n'
            'class = "intrinsic-only"\nowner = "none"\n'
        )
        rc, err = self._run()
        self.assertEqual(rc, 1)
        self.assertIn(
            "classified intrinsic-only but is a public intrinsic_wrapper symbol",
            err,
        )


class RepoGateTests(unittest.TestCase):
    def test_eradication_gate_passes_on_repo(self) -> None:
        result = subprocess.run(
            [sys.executable, "scripts/check/check-stdlib-eradication.py"],
            cwd=_REPO_ROOT,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("unregistered std declarations", result.stdout)


if __name__ == "__main__":
    unittest.main()
