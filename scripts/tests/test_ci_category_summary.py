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
            "| formatting | ${{ needs['quality-format'].result }} | [quality-format]",
            "| lint / quality contract | ${{ needs['quality-lint'].result }} | [quality-lint]",
            "| quick verification | ${{ needs['verify-quick'].result }} | [verify-quick]",
            "| verification / fixtures | ${{ needs['verification'].result }} | [verification]",
            "| bootstrap / selfhost parity | ${{ needs['selfhost'].result }} | [selfhost]",
            "| docs | ${{ needs['docs'].result }} | [docs]",
            "| editor-tooling / extension | ${{ needs['extension-tests'].result }} | [extension-tests]",
            "| release tag | ${{ needs['release-tag'].result }} | [release-tag]",
            "| final gate | ${{ needs['verify'].result }} | [verify]",
        ]

        for row in expected_rows:
            with self.subTest(row=row):
                self.assertIn(row, self.workflow)


if __name__ == "__main__":
    unittest.main()
