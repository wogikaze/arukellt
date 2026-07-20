"""Tests for verify-lane, verify-quick pools, and ARUKELLT_BUILD_DIR isolation."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import manager
from scripts.lib import build_paths

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
MANAGER = str(REPO_ROOT / "scripts" / "manager.py")


def _run(*args: str, extra_env: dict | None = None) -> subprocess.CompletedProcess:
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


class TestBuildPaths(unittest.TestCase):
    def test_default_build_dir_is_root_dot_build(self):
        with mock.patch.dict(os.environ, {}, clear=False):
            os.environ.pop("ARUKELLT_BUILD_DIR", None)
            root = Path("/tmp/arukellt-fake-root")
            self.assertEqual(build_paths.build_dir(root), (root / ".build").resolve())

    def test_env_overrides_build_dir_absolute(self):
        with tempfile.TemporaryDirectory() as tmp:
            override = Path(tmp) / "lane-a"
            with mock.patch.dict(os.environ, {"ARUKELLT_BUILD_DIR": str(override)}):
                root = Path("/tmp/arukellt-fake-root")
                self.assertEqual(build_paths.build_dir(root), override.resolve())
                self.assertEqual(
                    build_paths.runtime_lock_path(root),
                    override.resolve() / "selfhost-runtime.lock",
                )

    def test_distinct_build_dirs_yield_distinct_locks(self):
        with tempfile.TemporaryDirectory() as tmp:
            a = Path(tmp) / "a"
            b = Path(tmp) / "b"
            root = Path("/tmp/arukellt-fake-root")
            with mock.patch.dict(os.environ, {"ARUKELLT_BUILD_DIR": str(a)}):
                lock_a = build_paths.runtime_lock_path(root)
            with mock.patch.dict(os.environ, {"ARUKELLT_BUILD_DIR": str(b)}):
                lock_b = build_paths.runtime_lock_path(root)
            self.assertNotEqual(lock_a, lock_b)


class TestVerifyQuickPools(unittest.TestCase):
    def test_heavy_classification(self):
        self.assertTrue(
            manager._is_verify_quick_heavy_check(
                "T3 fixture WASM validation gate (#686)",
                "python3 scripts/check/check-t3-wasm-validate.py",
            )
        )
        self.assertFalse(
            manager._is_verify_quick_heavy_check(
                "ADR registry integrity",
                "python3 scripts/check/check-adrs.py",
            )
        )

    def test_verify_quick_dry_run_reports_pools(self):
        result = _run("verify", "quick", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr)
        combined = result.stdout + result.stderr
        self.assertIn("pools: static=", combined)
        self.assertIn("heavy=", combined)
        self.assertNotIn("orphan/stale file inventory", combined)


class TestVerifyLane(unittest.TestCase):
    def test_lane_dry_run(self):
        result = _run("verify", "lane", "--dry-run")
        self.assertEqual(result.returncode, 0, msg=result.stderr + result.stdout)
        combined = result.stdout + result.stderr
        self.assertIn("[lane] quality changed", combined)
        self.assertIn("no .ark files in changed set", combined)

    def test_lane_unknown_gate(self):
        result = _run("verify", "lane", "--dry-run", "--gate", "not-a-gate")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("invalid choice", (result.stderr + result.stdout).lower())


if __name__ == "__main__":
    unittest.main()
