"""Focused tests for the canonical CQ-12 finding model."""

import json
import tempfile
import unittest
from pathlib import Path

from scripts.quality.structure import (
    _compiler_dependency_direction_violations,
    _compiler_import_cycle_violations,
    _compiler_production_test_reachability_violations,
    collect_structure_report,
    quality_contract_findings,
    render_structure_text,
)


class TestStructureGraph(unittest.TestCase):
    def test_acyclic_import_fixture_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "src/compiler"
            compiler.mkdir(parents=True)
            (compiler / "a.ark").write_text("use b\n", encoding="utf-8")
            (compiler / "b.ark").write_text("fn b() {}\n", encoding="utf-8")
            self.assertEqual(_compiler_import_cycle_violations(root), [])

    def test_import_cycle_fixture(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "src/compiler"
            compiler.mkdir(parents=True)
            (compiler / "a.ark").write_text("use b\n", encoding="utf-8")
            (compiler / "b.ark").write_text("use a\n", encoding="utf-8")
            self.assertEqual(
                _compiler_import_cycle_violations(root),
                [["src/compiler/a.ark", "src/compiler/b.ark", "src/compiler/a.ark"]],
            )

    def test_dependency_direction_fixture(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "src/compiler/corehir/lower.ark"
            source.parent.mkdir(parents=True)
            source.write_text("use mir::body\n", encoding="utf-8")
            self.assertEqual(
                _compiler_dependency_direction_violations(root),
                [("src/compiler/corehir/lower.ark", 1, "use mir::body")],
            )

    def test_test_only_reachability_fixture(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "src/compiler"
            compiler.mkdir(parents=True)
            (compiler / "main.ark").write_text("use smoke_case\n", encoding="utf-8")
            (compiler / "smoke_case.ark").write_text("fn check() {}\n", encoding="utf-8")
            self.assertEqual(
                _compiler_production_test_reachability_violations(root),
                ["src/compiler/smoke_case.ark"],
            )


class TestStructureContract(unittest.TestCase):
    def _fixture(self, root: Path, rules: str, commands: str, workflow: str) -> None:
        (root / "docs/data").mkdir(parents=True)
        (root / "docs/adr").mkdir(parents=True)
        (root / ".github/workflows").mkdir(parents=True)
        (root / "docs/data/code-quality-rules.toml").write_text(rules, encoding="utf-8")
        (root / "docs/data/verification-commands.toml").write_text(commands, encoding="utf-8")
        (root / "docs/data/tooling-inventory.toml").write_text("families = []\n", encoding="utf-8")
        (root / "docs/adr/ADR-047-test.md").write_text("ステータス: **ACCEPTED**\n", encoding="utf-8")
        (root / ".github/workflows/ci.yml").write_text(workflow, encoding="utf-8")
        (root / "docs/data/ark-formatter-baseline.toml").write_text(
            'owner = "tooling"\nissue = 1\nremoval_condition = "fixed"\nrecheck_after = "2099-01-01"\n',
            encoding="utf-8",
        )
        (root / "issues/open").mkdir(parents=True)
        (root / "issues/open/1-test.md").write_text("# test\n", encoding="utf-8")

    def test_duplicate_rule_id_is_reported(self):
        rule = (
            '[[rules]]\nid="DUP"\nscope="repo"\ncategory="contract"\n'
            'enforcer="custom"\nseverity="error"\ngate="quick"\nautofix=false\n'
            'rationale="test"\nexception_policy="none"\nowner="tooling"\nadr="ADR-047"\n'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._fixture(root, rule + rule, "commands = []\n", "jobs:\n")
            messages = [item.message for item in quality_contract_findings(root)]
            self.assertIn("duplicate or empty rule id: DUP", messages)

    def test_missing_command_and_ci_job_are_reported(self):
        rule = (
            '[[rules]]\nid="CQ-TEST"\nscope="repo"\ncategory="contract"\n'
            'enforcer="custom"\nseverity="error"\ngate="quick"\nautofix=false\n'
            'rationale="test"\nexception_policy="none"\nowner="tooling"\nadr="ADR-047"\n'
            'commands=["missing"]\nci_jobs=["missing-job"]\n'
        )
        command = (
            '[[commands]]\nid="broken"\ncanonical="python3 scripts/manager.py absent"\n'
            'aliases=[]\nalias_status="none"\npurpose="test"\ncoverage=[]\n'
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "scripts").mkdir()
            (root / "scripts/manager.py").write_text("raise SystemExit(2)\n", encoding="utf-8")
            self._fixture(root, rule, command, "jobs:\n")
            messages = [item.message for item in quality_contract_findings(root)]
            self.assertIn("CQ-TEST: unknown command id missing", messages)
            self.assertIn("CQ-TEST: referenced CI job does not exist: missing-job", messages)
            self.assertIn("canonical command does not exist: python3 scripts/manager.py absent", messages)

    def test_text_and_json_use_same_finding_model(self):
        with tempfile.TemporaryDirectory() as tmp:
            report = collect_structure_report(Path(tmp), include_external=False)
            payload = report.to_dict()
            self.assertEqual(json.loads(json.dumps(payload)), payload)
            for finding in report.findings:
                self.assertIn(finding.rule_id, render_structure_text(report))


if __name__ == "__main__":
    unittest.main()
