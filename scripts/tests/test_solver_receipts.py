from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.common import ValidationError, sha256_file  # noqa: E402
from proof.solver_receipts import generate_solver_receipts, parse_solver_output  # noqa: E402
from proof.trust import validate_bound_proof  # noqa: E402


class SolverReceiptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.proof_dir = ROOT / "tests" / "proof"
        self.subject = self.proof_dir / "verified-core.json"
        self.solver_output = self.proof_dir / "solver-results.txt"
        self.toolchain = self.proof_dir / "toolchain" / "toolchain.json"

    def test_parse_raw_solver_results(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "solver.txt"
            path.write_text("unsat\nsat\nunknown\n(error bad)\n", encoding="utf-8")
            parsed = parse_solver_output(path)
        self.assertEqual(parsed["total"], 4)
        self.assertEqual(parsed["proved"], 1)
        self.assertEqual(parsed["refuted"], 1)
        self.assertEqual(parsed["unknown"], 1)
        self.assertEqual(parsed["errors"], 1)
        self.assertEqual(parsed["status"], "error")

    def test_unknown_solver_line_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "solver.txt"
            path.write_text("probably-unsat\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "unsupported result"):
                parse_solver_output(path)

    def test_generate_bound_proved_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest_path = root / "trust-manifest.json"
            receipt_path = root / "proof-receipt.json"
            manifest, receipt = generate_solver_receipts(
                self.subject,
                self.solver_output,
                self.toolchain,
                manifest_path,
                receipt_path,
            )
            validate_bound_proof(
                self.subject,
                manifest_path,
                receipt_path,
                self.solver_output,
            )
            toolchain = json.loads(self.toolchain.read_text(encoding="utf-8"))
            solver_path = self.toolchain.parent / toolchain["solver"]["executable"]
            self.assertEqual(manifest["solver"]["executable_sha256"], sha256_file(solver_path))
            self.assertEqual(receipt["status"], "proved")
            self.assertEqual(receipt["obligations"]["proved"], 1)

    def test_modified_solver_output_invalidates_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output_copy = root / "solver-results.txt"
            output_copy.write_text(self.solver_output.read_text(encoding="utf-8"), encoding="utf-8")
            manifest_path = root / "trust-manifest.json"
            receipt_path = root / "proof-receipt.json"
            generate_solver_receipts(
                self.subject,
                output_copy,
                self.toolchain,
                manifest_path,
                receipt_path,
            )
            output_copy.write_text("sat\n", encoding="utf-8")
            with self.assertRaisesRegex(ValidationError, "does not bind the supplied solver output"):
                validate_bound_proof(
                    self.subject,
                    manifest_path,
                    receipt_path,
                    output_copy,
                )

    def test_empty_solver_output_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "solver.txt"
            path.write_text("; no result\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "no obligations"):
                parse_solver_output(path)


if __name__ == "__main__":
    unittest.main()
