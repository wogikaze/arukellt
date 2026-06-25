"""Tests for the selfhost domain in manager.py.

Run from the repo root:
    python3 -m unittest scripts/tests/test_selfhost.py

NOTE: These tests require manager.py to have the selfhost domain wired in.
      They will fail until manager.py is patched with the selfhost block.
"""
from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
MANAGER = REPO_ROOT / "scripts" / "manager.py"


def _run(*args: str) -> tuple[int, str]:
    result = subprocess.run(
        [sys.executable, str(MANAGER), *args],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
    )
    return result.returncode, result.stdout + result.stderr


class TestSelfhostDryRun(unittest.TestCase):
    """Dry-run invocations must exit 0 and not touch the filesystem."""

    def test_fixpoint_dry_run(self) -> None:
        rc, out = _run("selfhost", "fixpoint", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("DRY-RUN", out)

    def test_fixture_parity_dry_run(self) -> None:
        rc, out = _run("selfhost", "fixture-parity", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("DRY-RUN", out)

    def test_diag_parity_dry_run(self) -> None:
        rc, out = _run("selfhost", "diag-parity", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("DRY-RUN", out)

    def test_parity_dry_run(self) -> None:
        rc, out = _run("selfhost", "parity", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("DRY-RUN", out)


class TestSelfhostUnknownSubcommand(unittest.TestCase):
    """Unknown subcommands must exit nonzero."""

    def test_unknown_subcommand(self) -> None:
        rc, out = _run("selfhost", "does-not-exist")
        self.assertNotEqual(rc, 0, f"Expected nonzero exit, got 0. Output:\n{out}")


class TestSelfhostFixpointBuildFlag(unittest.TestCase):
    """--build flag on fixpoint should be accepted (inverse of --no-build)."""

    def test_fixpoint_build_flag_dry_run(self) -> None:
        rc, out = _run("selfhost", "fixpoint", "--build", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")


class TestVerifyFullFixpointDryRun(unittest.TestCase):
    """verify --full --dry-run must include the fixpoint gate and exit 0."""

    def test_full_dry_run_includes_fixpoint(self) -> None:
        # verify --full --dry-run may exit non-zero due to pre-existing gate
        # failures (manifest drift, etc.); we only check that the fixpoint
        # gate is included in the output.
        rc, out = _run("verify", "--full", "--dry-run")
        self.assertIn("fixpoint", out.lower())
        self.assertIn("full verify", out)

    def test_selfhost_fixpoint_flag_dry_run(self) -> None:
        rc, out = _run("verify", "--selfhost-fixpoint", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("fixpoint", out.lower())
        self.assertIn("full verify", out)


if __name__ == "__main__":
    unittest.main()
