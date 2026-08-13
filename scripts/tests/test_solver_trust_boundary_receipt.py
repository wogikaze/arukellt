from __future__ import annotations

import copy
import hashlib
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.solver_trust_boundary_receipt import (  # noqa: E402
    REQUIRED_CAPABILITIES,
    REQUIRED_PRODUCERS,
    SolverTrustBoundaryReceiptError,
    validate_solver_trust_boundary_receipt,
)


class SolverTrustBoundaryReceiptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.boundary = self.root / "boundary.py"
        self.boundary.write_text("boundary\n", encoding="utf-8")
        digest = hashlib.sha256(self.boundary.read_bytes()).hexdigest()
        self.document = {
            "schema": "arukellt-solver-trust-boundary",
            "schema_version": 1,
            "status": "enforced",
            "primary_result": "arukellt-solver-result@1",
            "raw_solver_output_role": "evidence-only",
            "trust_manifest_policy": "embedded-and-file-bound",
            "proof_receipt_policy": "embedded-and-file-bound",
            "capabilities": sorted(REQUIRED_CAPABILITIES),
            "public_producers": sorted(REQUIRED_PRODUCERS),
            "failure_action": "no-valid-solver-result",
            "files": [{"path": "boundary.py", "sha256": digest}],
        }

    def validate(self, document: object | None = None):
        return validate_solver_trust_boundary_receipt(
            self.document if document is None else document,
            root=self.root,
        )

    def test_accepts_complete_receipt(self) -> None:
        self.assertEqual(self.validate()["status"], "enforced")

    def test_stale_file_is_rejected(self) -> None:
        self.boundary.write_text("changed\n", encoding="utf-8")
        with self.assertRaisesRegex(SolverTrustBoundaryReceiptError, "digest mismatch"):
            self.validate()

    def test_missing_capability_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        document["capabilities"].pop()
        with self.assertRaisesRegex(SolverTrustBoundaryReceiptError, "capability set mismatch"):
            self.validate(document)

    def test_duplicate_path_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        document["files"].append(copy.deepcopy(document["files"][0]))
        with self.assertRaisesRegex(SolverTrustBoundaryReceiptError, "duplicate path"):
            self.validate(document)

    def test_detached_manifest_policy_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        document["trust_manifest_policy"] = "external-only"
        with self.assertRaisesRegex(SolverTrustBoundaryReceiptError, "TrustManifest policy mismatch"):
            self.validate(document)


if __name__ == "__main__":
    unittest.main()
