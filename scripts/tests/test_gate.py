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
    _run_one_fixture,
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

    def test_runtime_selection_excludes_native_cpp_only_contracts(self):
        fixtures = [
            "native_cpp_public/panic_message.ark",
            "native_cpp_public/process_exit_7.ark",
            "stdlib_io/read_stdin_empty.ark",
        ]
        with patch.dict(os.environ, {"ARUKELLT_FIXTURE_EXECUTE_ALL": "1"}):
            selected = _select_smoke_fixtures(Path("/tmp"), fixtures)
        self.assertEqual(selected, {"stdlib_io/read_stdin_empty.ark"})

    def test_worker_count_honors_environment(self):
        with patch.dict(os.environ, {"ARUKELLT_FIXTURE_WORKERS": "7"}):
            self.assertEqual(_worker_count(), 7)
        with patch.dict(os.environ, {"ARUKELLT_FIXTURE_WORKERS": "0"}):
            self.assertEqual(_worker_count(), 1)

    def test_full_manifest_preserves_fixture_floor(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "compiler.wasm"
            compiler.write_bytes(b"compiler")
            fixtures = [f"domain/{index}.ark" for index in range(9)]

            def run_locked(function, _root):
                return function()

            with patch.object(selfhost_checks, "_find_pinned_wasm", return_value=compiler), \
                 patch.object(selfhost_checks, "_find_wasmtime", return_value="wasmtime"), \
                 patch.object(selfhost_checks, "_find_wasm_tools", return_value="wasm-tools"), \
                 patch.object(
                     selfhost_checks,
                     "_ensure_current_selfhost",
                     return_value=(compiler, ""),
                 ), patch.object(
                     selfhost_checks,
                     "_ensure_aot_cwasm",
                     return_value=compiler,
                 ), patch.object(
                     selfhost_checks,
                     "_with_runtime_lock",
                     side_effect=run_locked,
                 ), patch.object(
                     selfhost_checks,
                     "_load_manifest_fixtures",
                     return_value=(fixtures, ""),
                 ):
                rc, output = run_fixture_parity(root, dry_run=False)

            self.assertEqual(rc, 1)
            self.assertIn("fewer than 10", output)

    def test_golden_accepts_expected_nonzero_runtime_output(self):
        fixture = "sample/exit.ark"
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fixture_path = root / "tests" / "fixtures" / fixture
            fixture_path.parent.mkdir(parents=True)
            fixture_path.write_text("fn main() {}\n", encoding="utf-8")
            fixture_path.with_suffix(".expected").write_text("expected\n", encoding="utf-8")
            compiler = root / "compiler.wasm"
            compiler.write_bytes(b"compiler")
            component = root / "component.wasm"
            component.write_bytes(b"component")
            completed = subprocess.CompletedProcess(
                args=["wasmtime"], returncode=1, stdout="expected\n", stderr=""
            )

            def compile_fixture(_root, _wasmtime, _compiler, _fixture, out_rel):
                output = root / out_rel
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_bytes(b"wasm")
                return 0, ""

            with patch(
                "selfhost.fixture_parity._compile_current_fixture",
                side_effect=compile_fixture,
            ), patch(
                "selfhost.fixture_parity._validate_wasm",
                return_value=(0, ""),
            ), patch(
                "selfhost.fixture_parity._package_current_for_execution",
                return_value=(component, ""),
            ), patch.object(
                selfhost_checks,
                "_fixture_runtime_args",
                return_value=([], False),
            ), patch.object(
                selfhost_checks,
                "_wasm_run_argv",
                return_value=["wasmtime"],
            ), patch.object(
                selfhost_checks,
                "_run",
                return_value=completed,
            ):
                result = _run_one_fixture(
                    root,
                    "wasmtime",
                    "wasm-tools",
                    compiler,
                    fixture,
                    True,
                    root / "packages",
                )

            self.assertTrue(result.ok)


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
