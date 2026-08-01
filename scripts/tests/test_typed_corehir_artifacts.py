from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.common import ValidationError  # noqa: E402
from proof.typed_corehir import validate_document  # noqa: E402


class TypedCoreHirArtifactTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        fixture = json.loads((ROOT / "tests" / "proof" / "typed-corehir.json").read_text())
        fixture["schema"] = "arukellt-typed-corehir"
        cls.document = fixture

    def test_valid_typed_corehir(self) -> None:
        checked = validate_document(copy.deepcopy(self.document))
        self.assertEqual(checked["schema"], "arukellt-typed-corehir")
        self.assertEqual(checked["functions"][0]["body"]["root_expr_id"], 0)

    def test_rejects_verified_core_identity(self) -> None:
        document = copy.deepcopy(self.document)
        document["schema"] = "arukellt-verified-core"
        with self.assertRaisesRegex(ValidationError, "expected 'arukellt-typed-corehir'"):
            validate_document(document)

    def test_rejects_abi_representation_mismatch(self) -> None:
        document = copy.deepcopy(self.document)
        document["functions"][0]["abi"]["parameters"][0]["wasm"] = ["i64"]
        with self.assertRaisesRegex(ValidationError, "ABI representation must match type table"):
            validate_document(document)

    def test_rejects_unknown_child(self) -> None:
        document = copy.deepcopy(self.document)
        document["functions"][0]["body"]["expressions"][0]["children"].append(99)
        with self.assertRaisesRegex(ValidationError, "unknown expression id"):
            validate_document(document)

    def test_rejects_unreachable_expression(self) -> None:
        document = copy.deepcopy(self.document)
        document["functions"][0]["body"]["expressions"][0]["children"] = [1]
        with self.assertRaisesRegex(ValidationError, "unreachable expression"):
            validate_document(document)


if __name__ == "__main__":
    unittest.main()
