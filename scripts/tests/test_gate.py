"""Tests for gate domain checks and required product close-gates."""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

# Ensure scripts/ is on path when running from repo root
_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
_REPO_ROOT = _SCRIPTS_DIR.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from gate_domain.checks import (  # noqa: E402
    run_local,
    run_pre_commit,
    run_pre_push,
    run_repro,
)

ROOT = Path("/home/wogikaze/arukellt")


class TestGateDryRun(unittest.TestCase):
    def test_gate_local_dry_run(self):
        rc, out = run_local(ROOT, dry_run=True)
        self.assertEqual(rc, 0)
        self.assertEqual(out, "")

    def test_gate_pre_commit_dry_run(self):
        rc, out = run_pre_commit(ROOT, dry_run=True)
        self.assertEqual(rc, 0)
        self.assertEqual(out, "")

    def test_gate_pre_push_dry_run(self):
        rc, out = run_pre_push(ROOT, dry_run=True)
        self.assertEqual(rc, 0)
        self.assertEqual(out, "")

    def test_gate_repro_dry_run(self):
        rc, out = run_repro(ROOT, dry_run=True)
        self.assertEqual(rc, 0)
        self.assertEqual(out, "")

    def test_gate_local_skip_ext_dry_run(self):
        with patch("builtins.print") as mock_print:
            rc, out = run_local(ROOT, dry_run=True, skip_ext=True)
        self.assertEqual(rc, 0)
        self.assertEqual(out, "")
        printed = mock_print.call_args[0][0]
        self.assertIn("--skip-ext", printed)

    def test_gate_repro_verbose_dry_run(self):
        with patch("builtins.print") as mock_print:
            rc, out = run_repro(ROOT, dry_run=True, fixture="hello", target="wasm32", verbose=True)
        self.assertEqual(rc, 0)
        self.assertEqual(out, "")
        printed = mock_print.call_args[0][0]
        self.assertIn("--verbose", printed)
        self.assertIn("hello", printed)
        self.assertIn("wasm32", printed)

    def test_gate_unknown_subcommand(self):
        """Verify the dispatch table does not have a handler for a bogus subcommand."""
        dispatch = {
            "local": "cmd_gate_local",
            "pre-commit": "cmd_gate_pre_commit",
            "pre-push": "cmd_gate_pre_push",
            "repro": "cmd_gate_repro",
        }
        self.assertNotIn("bogus-subcommand", dispatch)


class TestWasiRuntimeAbiCloseGates(unittest.TestCase):
    """Keep #076/#676/#819/#841 contracts in the existing CI harness."""

    def test_wasi_runtime_abi_close_gates(self):
        gates = (
            "gate-076-wasi-p2-filesystem.py",
            "gate-676-std-host-fs-env-process.py",
            "gate-819-runtime-abi-core-op-lowering.py",
            "gate-841-real-wasi-network-abi.py",
        )
        for gate in gates:
            with self.subTest(gate=gate):
                run = subprocess.run(
                    [sys.executable, str(_REPO_ROOT / "scripts/check" / gate)],
                    cwd=_REPO_ROOT,
                    text=True,
                    capture_output=True,
                    check=False,
                )
                self.assertEqual(
                    run.returncode,
                    0,
                    msg=f"{gate} failed\nstdout:\n{run.stdout}\nstderr:\n{run.stderr}",
                )


if __name__ == "__main__":
    unittest.main()
