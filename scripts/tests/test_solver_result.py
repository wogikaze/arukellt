from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.solver_receipts import generate_solver_receipts  # noqa: E402
from proof.solver_result import (  # noqa: E402
    SolverResultError,
    create_solver_result,
    validate_solver_result,
    validate_solver_result_file,
    write_solver_result,
)


class SolverResultTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        proof = ROOT / "tests" / "proof"
        self.subject = proof / "verified-core.json"
        self.toolchain = proof / "toolchain" / "toolchain.json"
        self.input = self.root / "solver-input.smt2"
        self.input.write_text((proof / "solver-input.smt2").read_text(), encoding="utf-8")
        self.output = self.root / "solver-output.txt"
        self.output.write_text((proof / "solver-results.txt").read_text(), encoding="utf-8")
        self.manifest = self.root / "trust-manifest.json"
        self.receipt = self.root / "proof-receipt.json"
        generate_solver_receipts(
            self.subject,
            self.output,
            self.toolchain,
            self.manifest,
            self.receipt,
        )
        self.document = create_solver_result(
            subject_path=self.subject,
            solver_input_path=self.input,
            toolchain_path=self.toolchain,
            solver_output_path=self.output,
            trust_manifest_path=self.manifest,
            proof_receipt_path=self.receipt,
            execution_mode="captured-output",
            process_returncode=None,
            timed_out=False,
        )
        self.result = self.root / "solver-result.json"
        write_solver_result(self.document, self.result)

    def validate(self, document: object | None = None):
        if document is None:
            return validate_solver_result_file(
                self.result,
                subject_path=self.subject,
                solver_input_path=self.input,
                toolchain_path=self.toolchain,
                solver_output_path=self.output,
                trust_manifest_path=self.manifest,
                proof_receipt_path=self.receipt,
            )
        return validate_solver_result(
            document,
            subject_path=self.subject,
            solver_input_path=self.input,
            toolchain_path=self.toolchain,
            solver_output_path=self.output,
            trust_manifest_path=self.manifest,
            proof_receipt_path=self.receipt,
        )

    def test_complete_result_carries_manifest_and_receipt(self) -> None:
        validated = self.validate()
        self.assertEqual(validated["schema"], "arukellt-solver-result")
        self.assertEqual(validated["schema_version"], 1)
        self.assertEqual(validated["status"], "proved")
        self.assertEqual(validated["trust_manifest"], json.loads(self.manifest.read_text()))
        self.assertEqual(validated["proof_receipt"], json.loads(self.receipt.read_text()))

    def test_stale_solver_input_is_rejected(self) -> None:
        self.input.write_text("changed input\n", encoding="utf-8")
        with self.assertRaisesRegex(SolverResultError, "solver_input_sha256: digest mismatch"):
            self.validate()

    def test_stale_solver_output_is_rejected(self) -> None:
        self.output.write_text("sat\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "solver output"):
            self.validate()

    def test_stale_toolchain_is_rejected(self) -> None:
        copied = self.root / "toolchain.json"
        copied.write_text(self.toolchain.read_text() + "\n", encoding="utf-8")
        document = copy.deepcopy(self.document)
        with self.assertRaisesRegex(SolverResultError, "toolchain_sha256: digest mismatch"):
            validate_solver_result(
                document,
                subject_path=self.subject,
                solver_input_path=self.input,
                toolchain_path=copied,
                solver_output_path=self.output,
                trust_manifest_path=self.manifest,
                proof_receipt_path=self.receipt,
            )

    def test_embedded_manifest_substitution_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        document["trust_manifest"]["solver"]["version"] = "substituted"
        with self.assertRaisesRegex(SolverResultError, "embedded manifest differs"):
            self.validate(document)

    def test_embedded_receipt_substitution_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        document["proof_receipt"]["obligations"]["proved"] = 0
        document["proof_receipt"]["obligations"]["unknown"] = 1
        document["proof_receipt"]["status"] = "unknown"
        with self.assertRaisesRegex(SolverResultError, "embedded receipt differs"):
            self.validate(document)

    def test_top_level_status_cannot_disagree_with_receipt(self) -> None:
        document = copy.deepcopy(self.document)
        document["status"] = "unknown"
        with self.assertRaisesRegex(SolverResultError, "receipt status mismatch"):
            self.validate(document)

    def test_proved_process_result_requires_zero_exit(self) -> None:
        document = copy.deepcopy(self.document)
        document["execution"] = {
            "mode": "solver-process",
            "returncode": 2,
            "timed_out": False,
        }
        with self.assertRaisesRegex(SolverResultError, "nonzero solver exit"):
            self.validate(document)

    def test_captured_output_cannot_claim_process_exit(self) -> None:
        document = copy.deepcopy(self.document)
        document["execution"]["returncode"] = 0
        with self.assertRaisesRegex(SolverResultError, "captured output requires null"):
            self.validate(document)

    def test_missing_manifest_field_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        del document["trust_manifest"]
        with self.assertRaisesRegex(SolverResultError, "field set mismatch"):
            self.validate(document)


if __name__ == "__main__":
    unittest.main()
