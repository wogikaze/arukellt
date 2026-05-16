"""Behavioral contract tests for scripts.verify.harness."""

from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import unittest

from scripts.verify.harness import Harness


class TestHarnessFailureContext(unittest.TestCase):
    def test_check_fail_prints_category_command_and_primary_path(self):
        h = Harness(repo_root=Path("."))
        buf = StringIO()

        with redirect_stdout(buf):
            h.check_fail(
                "fixture harness",
                category="fixture",
                command="cargo test -p arukellt --test harness",
                primary_path="tests/fixtures/manifest.txt",
            )

        output = buf.getvalue()
        self.assertIn("fixture harness", output)
        self.assertIn("category: fixture", output)
        self.assertIn("command: cargo test -p arukellt --test harness", output)
        self.assertIn("primary path: tests/fixtures/manifest.txt", output)


if __name__ == "__main__":
    unittest.main()
