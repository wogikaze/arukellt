from __future__ import annotations

import copy
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.common import ValidationError  # noqa: E402
from proof.trust import (  # noqa: E402
    check_release_policy,
    validate_proof_receipt,
    validate_release_policy,
    validate_translation_receipt,
    validate_trust_manifest,
)
from proof.verified_core import validate_document as validate_verified_core  # noqa: E402


class FormalArtifactTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.proof_dir = ROOT / "tests" / "proof"
        cls.verified_core = json.loads((cls.proof_dir / "verified-core.json").read_text())
        cls.manifest = json.loads((cls.proof_dir / "trust-manifest.json").read_text())
        cls.receipt = json.loads((cls.proof_dir / "proof-receipt.json").read_text())
        cls.translation = json.loads((cls.proof_dir / "translation-receipt.json").read_text())

    def test_valid_verified_core(self) -> None:
        document = validate_verified_core(copy.deepcopy(self.verified_core))
        self.assertEqual(document["functions"][0]["signature"]["return_type_id"], 1)

    def test_verified_core_rejects_missing_representation(self) -> None:
        document = copy.deepcopy(self.verified_core)
        del document["types"][1]["representation"]
        with self.assertRaisesRegex(ValidationError, "missing field.*representation"):
            validate_verified_core(document)

    def test_verified_core_rejects_abi_signature_mismatch(self) -> None:
        document = copy.deepcopy(self.verified_core)
        document["functions"][0]["abi"]["parameters"][0]["type_id"] = 2
        with self.assertRaisesRegex(ValidationError, "ABI type must match signature"):
            validate_verified_core(document)

    def test_verified_core_rejects_return_type_mismatch(self) -> None:
        document = copy.deepcopy(self.verified_core)
        document["functions"][0]["body"]["blocks"][0]["terminator"]["value"] = {
            "kind": "constant",
            "type_id": 2,
            "value": True,
        }
        with self.assertRaisesRegex(ValidationError, "return type must match signature"):
            validate_verified_core(document)

    def test_valid_trust_manifest(self) -> None:
        self.assertEqual(validate_trust_manifest(self.manifest)["solver"]["name"], "z3")

    def test_trust_manifest_rejects_unknown_field(self) -> None:
        document = copy.deepcopy(self.manifest)
        document["implicit_trust"] = True
        with self.assertRaisesRegex(ValidationError, "unknown field"):
            validate_trust_manifest(document)

    def test_proved_receipt_must_prove_every_obligation(self) -> None:
        document = copy.deepcopy(self.receipt)
        document["obligations"]["proved"] = 0
        document["obligations"]["unknown"] = 1
        with self.assertRaisesRegex(ValidationError, "proved requires every obligation"):
            validate_proof_receipt(document)

    def test_valid_translation_receipt(self) -> None:
        self.assertEqual(validate_translation_receipt(self.translation)["status"], "validated")

    def test_translation_validated_must_cover_every_obligation(self) -> None:
        document = copy.deepcopy(self.translation)
        document["obligations"]["validated"] = 0
        document["obligations"]["unknown"] = 1
        with self.assertRaisesRegex(ValidationError, "validated requires every obligation"):
            validate_translation_receipt(document)

    def test_optional_policy_records_unmet_gates(self) -> None:
        policy = json.loads((ROOT / "release" / "proof-policy.json").read_text())
        checked = validate_release_policy(policy)
        self.assertEqual(checked["mode"], "proof-optional")
        self.assertFalse(checked["hard_gates"]["typed_verified_core_emission"])

    def test_required_policy_rejects_unmet_gate(self) -> None:
        policy = json.loads((self.proof_dir / "proof-required-policy.json").read_text())
        policy["hard_gates"]["optimizer_translation_validation"] = False
        with self.assertRaisesRegex(ValidationError, "unmet gate"):
            validate_release_policy(policy)

    def test_required_policy_checks_bound_receipts(self) -> None:
        result = check_release_policy(self.proof_dir / "proof-required-policy.json")
        self.assertEqual(result, ("proof-required", 1))

    def test_required_policy_rejects_digest_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            temp = Path(directory)
            for source in self.proof_dir.iterdir():
                if source.is_file():
                    shutil.copyfile(source, temp / source.name)
            receipt_path = temp / "proof-receipt.json"
            receipt = json.loads(receipt_path.read_text())
            receipt["subject"]["sha256"] = "0" * 64
            receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
            with self.assertRaisesRegex(ValidationError, "does not bind"):
                check_release_policy(temp / "proof-required-policy.json")


if __name__ == "__main__":
    unittest.main()
