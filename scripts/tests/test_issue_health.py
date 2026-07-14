from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check" / "check-issue-health.py"
SPEC = importlib.util.spec_from_file_location("issue_health", SCRIPT)
assert SPEC and SPEC.loader
issue_health = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = issue_health
SPEC.loader.exec_module(issue_health)


def _write_issue(path: Path, issue_id: str, status: str = "open") -> None:
    path.write_text(
        f"---\nStatus: {status}\nID: {issue_id}\n---\n# {issue_id} test\n",
        encoding="utf-8",
    )


class IssueHealthTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.open_dir = self.root / "open"
        self.done_dir = self.root / "done"
        self.open_dir.mkdir()
        self.done_dir.mkdir()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _run(self) -> int:
        return issue_health.check_issues(self.open_dir, self.done_dir, fix=False)

    def test_pass_when_clean(self) -> None:
        _write_issue(self.open_dir / "100-a.md", "100", "open")
        _write_issue(self.done_dir / "099-a.md", "099", "done")
        self.assertEqual(self._run(), 0)

    def test_duplicate_id_within_open(self) -> None:
        _write_issue(self.open_dir / "200-a.md", "200", "open")
        _write_issue(self.open_dir / "200-b.md", "200", "open")
        self.assertNotEqual(self._run(), 0)

    def test_duplicate_id_within_done(self) -> None:
        _write_issue(self.done_dir / "300-a.md", "300", "done")
        _write_issue(self.done_dir / "300-b.md", "300", "done")
        self.assertNotEqual(self._run(), 0)

    def test_duplicate_id_across_dirs(self) -> None:
        _write_issue(self.open_dir / "400-a.md", "400", "open")
        _write_issue(self.done_dir / "400-b.md", "400", "done")
        self.assertNotEqual(self._run(), 0)

    def test_filename_frontmatter_id_mismatch(self) -> None:
        (self.open_dir / "500-a.md").write_text(
            "---\nStatus: open\nID: 501\n---\n# test\n", encoding="utf-8"
        )
        self.assertNotEqual(self._run(), 0)

    def test_status_mismatch(self) -> None:
        _write_issue(self.open_dir / "600-a.md", "600", "done")
        self.assertNotEqual(self._run(), 0)

    def test_dead_dependency(self) -> None:
        (self.open_dir / "700-a.md").write_text(
            '---\nStatus: open\nID: 700\nDepends on: "999"\n---\n# test\n',
            encoding="utf-8",
        )
        self.assertNotEqual(self._run(), 0)

    def test_unchecked_in_done(self) -> None:
        (self.done_dir / "800-a.md").write_text(
            "---\nStatus: done\nID: 800\n---\n# test\n- [ ] unchecked item\n",
            encoding="utf-8",
        )
        self.assertNotEqual(self._run(), 0)


if __name__ == "__main__":
    unittest.main()
