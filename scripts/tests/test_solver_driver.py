from __future__ import annotations

import json
import os
import stat
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.solver_driver import run_solver_and_generate_receipts  # noqa: E402
from proof.trust import validate_bound_proof  # noqa: E402


class SolverDriverTests(unittest.TestCase):
    def setUp(self) -> None:
        self.proof_dir = ROOT / "tests" / "proof"
        self.subject = self.proof_dir / "verified-core.json"
        self.toolchain = self.proof_dir / "toolchain" / "toolchain.json"
        self.solver = self.proof_dir / "toolchain" / "fake_solver.py"
        self.original_mode = stat.S_IMODE(self.solver.stat().st_mode)
        self.solver.chmod(self.original_mode | stat.S_IXUSR)
        self.addCleanup(self.solver.chmod, self.original_mode)

    def run_fixture(self, solver_input: str):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        input_path = root / "input.smt2"
        input_path.write_text(solver_input, encoding="utf-8")
        output_path = root / "solver-output.txt"
        manifest_path = root / "trust-manifest.json"
        receipt_path = root / "proof-receipt.json"
        result = run_solver_and_generate_receipts(
            self.subject,
            input_path,
            self.toolchain,
            output_path,
            manifest_path,
            receipt_path,
        )
        return result, output_path, manifest_path, receipt_path

    def test_proved_solver_run_is_bound(self) -> None:
        result, output, manifest, receipt = self.run_fixture("prove\n")
        self.assertEqual(result.process_returncode, 0)
        self.assertEqual(result.proof_status, "proved")
        self.assertEqual(output.read_text(encoding="utf-8"), "unsat\n")
        validate_bound_proof(self.subject, manifest, receipt, output)

    def test_unknown_is_not_promoted_to_proved(self) -> None:
        result, output, _, receipt = self.run_fixture("not-modelled\n")
        document = json.loads(receipt.read_text(encoding="utf-8"))
        self.assertEqual(result.process_returncode, 0)
        self.assertEqual(result.proof_status, "unknown")
        self.assertEqual(document["obligations"]["unknown"], 1)
        self.assertEqual(output.read_text(encoding="utf-8"), "unknown\n")

    def test_nonzero_solver_exit_produces_error_receipt(self) -> None:
        result, output, _, receipt = self.run_fixture("error\n")
        document = json.loads(receipt.read_text(encoding="utf-8"))
        self.assertEqual(result.process_returncode, 2)
        self.assertEqual(result.proof_status, "error")
        self.assertEqual(document["obligations"]["errors"], 1)
        self.assertIn("(error simulated)", output.read_text(encoding="utf-8"))

    def test_non_executable_solver_is_rejected(self) -> None:
        self.solver.chmod(self.original_mode & ~stat.S_IXUSR & ~stat.S_IXGRP & ~stat.S_IXOTH)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "input.smt2"
            input_path.write_text("prove\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "not executable"):
                run_solver_and_generate_receipts(
                    self.subject,
                    input_path,
                    self.toolchain,
                    root / "solver-output.txt",
                    root / "trust-manifest.json",
                    root / "proof-receipt.json",
                )


if __name__ == "__main__":
    unittest.main()
