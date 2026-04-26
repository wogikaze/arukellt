"""Behavioral contract tests for scripts/manager.py."""

import subprocess
import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent  # ~/arukellt
MANAGER = str(REPO_ROOT / "scripts" / "manager.py")


def _run(*args: str, extra_env: dict | None = None) -> subprocess.CompletedProcess:
    import os

    env = os.environ.copy()
    if extra_env:
        env.update(extra_env)
    return subprocess.run(
        [sys.executable, MANAGER, *args],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        env=env,
    )


class TestVerifyQuick(unittest.TestCase):
    def test_verify_quick_exit_code_type(self):
        result = _run("verify", "quick")
        self.assertIsNotNone(result.returncode)
        self.assertIsInstance(result.returncode, int)
        self.assertIn(result.returncode, (0, 1))

    def test_verify_quick_stdout_has_summary(self):
        result = _run("verify", "quick")
        combined = result.stdout + result.stderr
        self.assertTrue(
            any(
                token in combined
                for token in ("Summary", "checks passed", "checks failed")
            ),
            msg=f"Expected summary output, got:\n{combined[:2000]}",
        )


class TestVerifyDryRun(unittest.TestCase):
    def test_verify_fixtures_dry_run(self):
        result = _run("verify", "fixtures", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)

    def test_verify_size_dry_run(self):
        result = _run("verify", "size", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)

    def test_verify_wat_dry_run(self):
        result = _run("verify", "wat", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)

    def test_verify_component_dry_run(self):
        result = _run("verify", "component", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)


class TestEnvBehavior(unittest.TestCase):
    def test_env_arukellt_bin_honored(self):
        """ARUKELLT_BIN=/nonexistent with --dry-run must still exit 0."""
        result = _run("verify", "size", "--dry-run", extra_env={"ARUKELLT_BIN": "/nonexistent"})
        self.assertEqual(result.returncode, 0, msg=result.stderr)


class TestUnknownSubcommand(unittest.TestCase):
    def test_unknown_subcommand_exits_nonzero(self):
        result = _run("verify", "bogus")
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
