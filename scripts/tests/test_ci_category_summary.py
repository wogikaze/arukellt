"""Contract tests for the CI category summary job."""

from pathlib import Path
import unittest


REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CI_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"


class TestCiCategorySummary(unittest.TestCase):
    def setUp(self):
        self.workflow = CI_WORKFLOW.read_text()

    def test_summary_job_emits_job_summary_and_artifact(self):
        self.assertIn("ci-category-summary:", self.workflow)
        self.assertIn("name: \"CI category summary\"", self.workflow)
        self.assertIn("if: always()", self.workflow)
        self.assertIn("GITHUB_STEP_SUMMARY", self.workflow)
        self.assertIn("actions/upload-artifact@v4", self.workflow)
        self.assertIn("ci-category-summary-${{ github.run_id }}", self.workflow)

    def test_summary_lists_named_categories_and_responsible_jobs(self):
        expected_rows = [
            "| unit | ${{ needs['unit-tests'].result }} | [unit-tests]",
            "| verification-harness | ${{ needs['verification-harness-quick'].result }} | [verification-harness-quick]",
            "| fixture / target-contract (T3 primary) | ${{ needs['fixture-primary'].result }} | [fixture-primary]",
            "| fixture / target-contract (T1 supported) | ${{ needs['fixture-supported'].result }} | [fixture-supported]",
            "| integration | ${{ needs['integration'].result }} | [integration]",
            "| packaging | ${{ needs['packaging'].result }} | [packaging]",
            "| package-workspace | ${{ needs['verification-package-workspace'].result }} | [verification-package-workspace]",
            "| determinism | ${{ needs['determinism'].result }} | [determinism]",
            "| docs / size / WAT | ${{ needs['heavy-checks'].result }} | [heavy-checks]",
            "| component-interop | ${{ needs['component-interop'].result }} | [component-interop]",
            "| perf | ${{ needs['perf-baseline'].result }} | [perf-baseline]",
            "| bootstrap | ${{ needs['selfhost-bootstrap'].result }} | [selfhost-bootstrap]",
            "| selfhost-parity / CLI | ${{ needs['selfhost-cli-parity'].result }} | [selfhost-cli-parity]",
            "| selfhost-parity / diagnostics | ${{ needs['selfhost-diag-parity'].result }} | [selfhost-diag-parity]",
            "| diagnostics-snapshot | ${{ needs['fixture-primary'].result }} | [fixture-primary]",
            "| editor-tooling / extension | ${{ needs['extension-tests'].result }} | [extension-tests]",
            "| editor-tooling / LSP lifecycle | ${{ needs['verification-harness-quick'].result }} | [verification-harness-quick]",
            "| target-contract drift | ${{ needs['target-contract-drift-check'].result }} | [target-contract-drift-check]",
            "| final gate | ${{ needs['verify'].result }} | [verify]",
        ]

        for row in expected_rows:
            with self.subTest(row=row):
                self.assertIn(row, self.workflow)


if __name__ == "__main__":
    unittest.main()
