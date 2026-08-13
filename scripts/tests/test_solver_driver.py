from __future__ import annotations

import json
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
from proof.solver_result import validate_solver_result_file  # noqa: E402
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
        result_path = root / "solver-result.json"
        result = run_solver_and_generate_receipts(
            self.subject,
            input_path,
            self.toolchain,
            output_path,
            manifest_path,
            receipt_path,
            result_path,
        )
        return result, input_path, output_path, manifest_path, receipt_path, result_path

    def validate_result(
        self,
        input_path: Path,
        output_path: Path,
        manifest_path: Path,
        receipt_path: Path,
        result_path: Path,
    ) -> dict[str, object]:
        return validate_solver_result_file(
            result_path,
            subject_path=self.subject,
            solver_input_path=input_path,
            toolchain_path=self.toolchain,
            solver_output_path=output_path,
            trust_manifest_path=manifest_path,
            proof_receipt_path=receipt_path,
        )

    def test_proved_solver_run_is_bound(self) -> None:
        result, input_path, output, manifest, receipt, solver_result = self.run_fixture("prove\n")
        self.assertEqual(result.process_returncode, 0)
        self.assertEqual(result.proof_status, "proved")
        self.assertEqual(result.solver_result_path, solver_result)
        self.assertEqual(output.read_text(encoding="utf-8"), "unsat\n")
        validate_bound_proof(self.subject, manifest, receipt, output)
        document = self.validate_result(input_path, output, manifest, receipt, solver_result)
        self.assertEqual(document["execution"], {
            "mode": "solver-process",
            "returncode": 0,
            "timed_out": False,
        })
        self.assertEqual(document["trust_manifest"], json.loads(manifest.read_text()))
        self.assertEqual(document["proof_receipt"], json.loads(receipt.read_text()))

    def test_unknown_is_not_promoted_to_proved(self) -> None:
        result, input_path, output, manifest, receipt, solver_result = self.run_fixture("not-modelled\n")
        receipt_document = json.loads(receipt.read_text(encoding="utf-8"))
        self.assertEqual(result.process_returncode, 0)
        self.assertEqual(result.proof_status, "unknown")
        self.assertEqual(receipt_document["obligations"]["unknown"], 1)
        self.assertEqual(output.read_text(encoding="utf-8"), "unknown\n")
        result_document = self.validate_result(input_path, output, manifest, receipt, solver_result)
        self.assertEqual(result_document["status"], "unknown")

    def test_nonzero_solver_exit_produces_error_result(self) -> None:
        result, input_path, output, manifest, receipt, solver_result = self.run_fixture("error\n")
        receipt_document = json.loads(receipt.read_text(encoding="utf-8"))
        self.assertEqual(result.process_returncode, 2)
        self.assertEqual(result.proof_status, "error")
        self.assertEqual(receipt_document["obligations"]["errors"], 1)
        self.assertIn("(error simulated)", output.read_text(encoding="utf-8"))
        result_document = self.validate_result(input_path, output, manifest, receipt, solver_result)
        self.assertEqual(result_document["execution"]["returncode"], 2)
        self.assertEqual(result_document["status"], "error")

    def test_non_executable_solver_is_rejected_without_result(self) -> None:
        self.solver.chmod(self.original_mode & ~stat.S_IXUSR & ~stat.S_IXGRP & ~stat.S_IXOTH)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "input.smt2"
            input_path.write_text("prove\n", encoding="utf-8")
            result_path = root / "solver-result.json"
            with self.assertRaisesRegex(ValueError, "not executable"):
                run_solver_and_generate_receipts(
                    self.subject,
                    input_path,
                    self.toolchain,
                    root / "solver-output.txt",
                    root / "trust-manifest.json",
                    root / "proof-receipt.json",
                    result_path,
                )
            self.assertFalse(result_path.exists())


if __name__ == "__main__":
    unittest.main()
