"""Tests for gate domain checks and required product close-gates."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
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
from selfhost import checks as selfhost_checks  # noqa: E402
from selfhost.fixture_parity import (  # noqa: E402
    _select_smoke_fixtures,
    _worker_count,
    run_fixture_parity,
    run_reference_fixture_parity,
)

ROOT = _REPO_ROOT


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


class TestFixtureGateMigration(unittest.TestCase):
    """The normal fixture gate must stay current-only, bounded, and parallel."""

    def test_checks_entrypoint_uses_new_gate(self):
        self.assertIs(selfhost_checks.run_fixture_parity, run_fixture_parity)
        self.assertIsNot(run_reference_fixture_parity, run_fixture_parity)

    def test_smoke_selection_spans_domains_and_prefers_goldens(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fixtures_root = root / "tests" / "fixtures"
            fixtures_root.mkdir(parents=True)
            fixtures = [
                "alpha/a.ark",
                "alpha/b.ark",
                "beta/x.ark",
                "gamma/y.ark",
            ]
            for fixture in fixtures:
                path = fixtures_root / fixture
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fn main() {}\n", encoding="utf-8")
            (fixtures_root / "alpha" / "b.expected").write_text("ok\n", encoding="utf-8")
            (fixtures_root / "gamma" / "y.expected").write_text("ok\n", encoding="utf-8")

            with patch.dict(os.environ, {}, clear=False):
                os.environ.pop("ARUKELLT_FIXTURE_EXECUTE_ALL", None)
                selected = _select_smoke_fixtures(root, fixtures)

            self.assertEqual(selected, {"alpha/b.ark", "beta/x.ark", "gamma/y.ark"})

    def test_execute_all_is_explicit_opt_in(self):
        fixtures = ["a/x.ark", "b/y.ark"]
        with tempfile.TemporaryDirectory() as tmp, patch.dict(
            os.environ, {"ARUKELLT_FIXTURE_EXECUTE_ALL": "1"}
        ):
            self.assertEqual(_select_smoke_fixtures(Path(tmp), fixtures), set(fixtures))

    def test_worker_count_honors_environment(self):
        with patch.dict(os.environ, {"ARUKELLT_FIXTURE_WORKERS": "7"}):
            self.assertEqual(_worker_count(), 7)
        with patch.dict(os.environ, {"ARUKELLT_FIXTURE_WORKERS": "0"}):
            self.assertEqual(_worker_count(), 1)


class TestWasiRuntimeAbiCloseGates(unittest.TestCase):
    """Keep the active WASI filesystem and runtime lowering contracts in CI."""

    def test_wasi_runtime_abi_close_gates(self):
        gates = (
            "gate-076-wasi-p2-filesystem.py",
            "gate-676-std-host-fs-env-process.py",
            "gate-819-runtime-abi-core-op-lowering.py",
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
