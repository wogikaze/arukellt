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

from proof.typed_verified_core_receipt import REQUIRED_SEMANTIC_CHECKS, TypedVerifiedCoreReceiptError, validate_boundary_receipt


class TypedVerifiedCoreReceiptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.bound = self.root / "validator.py"
        self.bound.write_text("validator\n", encoding="utf-8")
        digest = hashlib.sha256(self.bound.read_bytes()).hexdigest()
        self.document = {
            "schema": "arukellt-typed-verified-core-boundary", "schema_version": 1, "status": "enforced",
            "source_schema": "arukellt-typed-corehir@1", "target_schema": "arukellt-verified-core@1",
            "converter": "arukellt-typed-corehir-converter-v3",
            "logical_integer_metadata": "explicit-bits-and-signedness", "type_name_semantics": "identity-only",
            "structural_validator": "verified_core.py@1", "semantic_validator": "verified_core_typed.py@2",
            "solver_adapter": "smtlib_typed_v1.py@1", "semantic_checks": sorted(REQUIRED_SEMANTIC_CHECKS),
            "failure_action": "reject-before-SMT-generation", "files": [{"path": "validator.py", "sha256": digest}],
        }

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_accepts_complete_hash_bound_receipt(self) -> None:
        self.assertEqual(validate_boundary_receipt(copy.deepcopy(self.document), root=self.root)["status"], "enforced")

    def test_rejects_stale_file_digest(self) -> None:
        self.bound.write_text("changed\n", encoding="utf-8")
        with self.assertRaisesRegex(TypedVerifiedCoreReceiptError, "digest mismatch"):
            validate_boundary_receipt(copy.deepcopy(self.document), root=self.root)

    def test_rejects_missing_semantic_check(self) -> None:
        document = copy.deepcopy(self.document); document["semantic_checks"].pop()
        with self.assertRaisesRegex(TypedVerifiedCoreReceiptError, "semantic_checks set mismatch"):
            validate_boundary_receipt(document, root=self.root)

    def test_rejects_duplicate_bound_path(self) -> None:
        document = copy.deepcopy(self.document); document["files"].append(copy.deepcopy(document["files"][0]))
        with self.assertRaisesRegex(TypedVerifiedCoreReceiptError, "duplicate path"):
            validate_boundary_receipt(document, root=self.root)


if __name__ == "__main__":
    unittest.main()
