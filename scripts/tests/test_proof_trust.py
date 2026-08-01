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

from proof.trust import (  # noqa: E402
    check_release_policy,
    validate_bound_receipt,
    validate_proof_receipt,
    validate_release_policy,
    validate_trust_manifest,
)


class ProofTrustTests(unittest.TestCase):
    def setUp(self) -> None:
        self.proof_ir_path = ROOT / "tests" / "proof-ir" / "basic.json"
        self.manifest_path = ROOT / "tests" / "proof" / "trust-manifest.json"
        self.receipt_path = ROOT / "tests" / "proof" / "proof-receipt.json"
        self.policy_path = ROOT / "tests" / "proof-release-policy.json"
        self.manifest = json.loads(self.manifest_path.read_text(encoding="utf-8"))
        self.receipt = json.loads(self.receipt_path.read_text(encoding="utf-8"))

    def test_valid_manifest(self) -> None:
        self.assertEqual(validate_trust_manifest(self.manifest)["schema_version"], 1)

    def test_valid_receipt(self) -> None:
        self.assertEqual(validate_proof_receipt(self.receipt)["status"], "proved")

    def test_bound_receipt_matches_files(self) -> None:
        _, manifest, receipt = validate_bound_receipt(
            self.proof_ir_path, self.manifest_path, self.receipt_path
        )
        self.assertEqual(manifest["solver"]["name"], "z3")
        self.assertEqual(receipt["obligations"]["proved"], 1)

    def test_manifest_rejects_unknown_fields(self) -> None:
        value = copy.deepcopy(self.manifest)
        value["implicit_trust"] = True
        with self.assertRaises(ValueError):
            validate_trust_manifest(value)

    def test_receipt_rejects_inconsistent_proved_status(self) -> None:
        value = copy.deepcopy(self.receipt)
        value["obligations"]["proved"] = 0
        value["obligations"]["unknown"] = 1
        with self.assertRaises(ValueError):
            validate_proof_receipt(value)

    def test_bound_receipt_rejects_digest_mismatch(self) -> None:
        value = copy.deepcopy(self.receipt)
        value["proof_ir_sha256"] = "0" * 64
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            path.write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaises(ValueError):
                validate_bound_receipt(self.proof_ir_path, self.manifest_path, path)

    def test_proof_required_policy_rejects_empty_receipts(self) -> None:
        with self.assertRaises(ValueError):
            validate_release_policy(
                {
                    "schema": "arukellt-proof-release-policy",
                    "schema_version": 1,
                    "mode": "proof-required",
                    "artifacts": [],
                }
            )

    def test_valid_proof_required_release(self) -> None:
        self.assertEqual(check_release_policy(self.policy_path), ("proof-required", 1))

    def test_optional_release_may_have_no_receipts(self) -> None:
        policy = validate_release_policy(
            {
                "schema": "arukellt-proof-release-policy",
                "schema_version": 1,
                "mode": "proof-optional",
                "artifacts": [],
            }
        )
        self.assertEqual(policy["artifacts"], [])


if __name__ == "__main__":
    unittest.main()
