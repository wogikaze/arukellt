from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check" / "check-comment-policy.py"
SPEC = importlib.util.spec_from_file_location("comment_policy", SCRIPT)
assert SPEC and SPEC.loader
comment_policy = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = comment_policy
SPEC.loader.exec_module(comment_policy)


class CommentPolicyTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "src/compiler/internal").mkdir(parents=True)
        (self.root / "std").mkdir()
        (self.root / "scripts").mkdir()
        (self.root / "std/manifest.toml").write_text(
            '[[functions]]\nname = "println"\ndoc_category = "io"\n',
            encoding="utf-8",
        )
        self.old_root = comment_policy.ROOT
        self.old_scan_roots = comment_policy.SCAN_ROOTS
        comment_policy.ROOT = self.root
        comment_policy.SCAN_ROOTS = (
            self.root / "src/compiler",
            self.root / "std",
            self.root / "scripts",
        )

    def tearDown(self) -> None:
        comment_policy.ROOT = self.old_root
        comment_policy.SCAN_ROOTS = self.old_scan_roots
        self.temp.cleanup()

    def test_api_classification_and_boundary_doc_contract(self) -> None:
        (self.root / "src/compiler/driver.ark").write_text(
            "/// Compiles source with the requested driver configuration.\n"
            "pub fn compile_source() {}\n",
            encoding="utf-8",
        )
        (self.root / "src/compiler/internal/query.ark").write_text(
            "pub fn cross_module_query() {}\n",
            encoding="utf-8",
        )
        findings, api = comment_policy.collect_findings()
        self.assertFalse([item for item in findings if item.severity == "error"])
        self.assertEqual((api.external, api.external_documented), (1, 1))
        self.assertEqual((api.subsystem_boundary, api.subsystem_boundary_documented), (1, 1))
        self.assertEqual(api.internal_cross_module, 1)

    def test_missing_boundary_doc_is_error_but_internal_pub_is_not(self) -> None:
        (self.root / "src/compiler/driver.ark").write_text("pub fn compile_source() {}\n", encoding="utf-8")
        (self.root / "src/compiler/internal/query.ark").write_text(
            "pub fn cross_module_query() {}\n",
            encoding="utf-8",
        )
        findings, _ = comment_policy.collect_findings()
        errors = [item for item in findings if item.rule_id == "CQ-API-001"]
        self.assertEqual([(item.path, item.line) for item in errors], [("src/compiler/driver.ark", 1)])

    def test_comment_findings_distinguish_hard_and_advisory_cases(self) -> None:
        (self.root / "src/compiler/internal/query.ark").write_text(
            "// Arukellt Selfhost - query helpers.\n"
            "// issue #715\n"
            "// let disabled = 1\n"
            "// let this prose explain the invariant\n"
            "// TODO untracked\n"
            "/// Detached contract.\n"
            "\n"
            "fn query() {}\n",
            encoding="utf-8",
        )
        findings, _ = comment_policy.collect_findings()
        by_rule = {item.rule_id: item.severity for item in findings}
        self.assertEqual(by_rule["CQ-DOC-001"], "error")
        self.assertEqual(by_rule["CQ-DOC-003"], "error")
        self.assertEqual(by_rule["CQ-DOC-004"], "error")
        self.assertEqual(by_rule["CQ-DOC-005"], "warning")
        self.assertEqual(by_rule["CQ-DOC-006"], "error")
        commented_code = [item for item in findings if item.rule_id == "CQ-DOC-004"]
        self.assertEqual(len(commented_code), 1)

    def test_structured_todo_allows_descriptive_quoted_removal_condition(self) -> None:
        (self.root / "src/compiler/internal/query.ark").write_text(
            '// TODO(#797 owner=compiler removal="registry becomes authoritative" recheck=2026-10-01)\n',
            encoding="utf-8",
        )
        findings, _ = comment_policy.collect_findings()
        self.assertFalse([item for item in findings if item.rule_id == "CQ-DOC-001"])


if __name__ == "__main__":
    unittest.main()
