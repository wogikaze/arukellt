from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check" / "check-semantic-debt.py"
SPEC = importlib.util.spec_from_file_location("semantic_debt", SCRIPT)
assert SPEC and SPEC.loader
semantic_debt = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = semantic_debt
SPEC.loader.exec_module(semantic_debt)


class SemanticDebtTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "src/compiler/fmt").mkdir(parents=True)
        (self.root / "std").mkdir()
        (self.root / "docs/data").mkdir(parents=True)
        (self.root / "issues/open").mkdir(parents=True)
        (self.root / "issues/done").mkdir(parents=True)
        (self.root / "issues/open/828-restore-import-block-span-record.md").write_text(
            "# 828\n",
            encoding="utf-8",
        )
        self.old_root = semantic_debt.ROOT
        self.old_allow = semantic_debt.ALLOWLIST_PATH
        self.old_scan = semantic_debt.SCAN_ROOTS
        semantic_debt.ROOT = self.root
        semantic_debt.ALLOWLIST_PATH = self.root / "docs/data/semantic-debt-allowlist.toml"
        semantic_debt.SCAN_ROOTS = (self.root / "src/compiler", self.root / "std")

    def tearDown(self) -> None:
        semantic_debt.ROOT = self.old_root
        semantic_debt.ALLOWLIST_PATH = self.old_allow
        semantic_debt.SCAN_ROOTS = self.old_scan
        self.temp.cleanup()

    def test_new_pack_without_allowlist_fails(self) -> None:
        (self.root / "src/compiler/fmt/sort_imports.ark").write_text(
            "fn pack() {\n    let base: i64 = i32_to_i64(1000000)\n"
            "    i64_to_i32(span / i32_to_i64(1000000))\n}\n",
            encoding="utf-8",
        )
        findings = semantic_debt.collect_findings(today=date(2026, 7, 17))
        self.assertTrue(
            any(f.rule_id == "i32-pair-pack-million" for f in findings),
            findings,
        )
        self.assertTrue(
            any(f.rule_id == "i32-pair-unpack-million" for f in findings),
            findings,
        )

    def test_allowlisted_pack_passes(self) -> None:
        (self.root / "src/compiler/fmt/sort_imports.ark").write_text(
            "fn pack() {\n    let base: i64 = i32_to_i64(1000000)\n}\n",
            encoding="utf-8",
        )
        semantic_debt.ALLOWLIST_PATH.write_text(
            '[[debt]]\n'
            'id = "pack"\n'
            'rule = "i32-pair-pack-million"\n'
            'path = "src/compiler/fmt/sort_imports.ark"\n'
            "issue = 828\n"
            'removal = "restore record"\n'
            'recheck = "2026-08-17"\n',
            encoding="utf-8",
        )
        findings = semantic_debt.collect_findings(today=date(2026, 7, 17))
        errors = [f for f in findings if f.severity == "error"]
        self.assertEqual(errors, [])

    def test_time_math_not_flagged(self) -> None:
        (self.root / "src/compiler/fmt/clock.ark").write_text(
            "fn ms(diff: i64) -> i32 {\n    i64_to_i32(diff / 1000000i64)\n}\n",
            encoding="utf-8",
        )
        findings = semantic_debt.collect_findings(today=date(2026, 7, 17))
        self.assertEqual([f for f in findings if f.severity == "error"], [])

    def test_workaround_comment_needs_structured_todo(self) -> None:
        (self.root / "src/compiler/fmt/hack.ark").write_text(
            "// temporary workaround for broken returns\nfn f() {}\n",
            encoding="utf-8",
        )
        findings = semantic_debt.collect_findings(today=date(2026, 7, 17))
        self.assertTrue(
            any(f.rule_id == "untracked-workaround-comment" for f in findings),
            findings,
        )


if __name__ == "__main__":
    unittest.main()
