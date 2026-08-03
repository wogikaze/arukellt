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

from proof.typed_corehir_typed_convert import convert_typed_document  # noqa: E402
from proof.verified_core_typed import (  # noqa: E402
    TypedVerifiedCoreError,
    validate_typed_document,
)


class TypedVerifiedCoreTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        source = json.loads(
            (ROOT / "tests" / "proof" / "typed-corehir.json").read_text()
        )
        cls.document = convert_typed_document(source)

    def validate(self, document: dict[str, object]) -> dict[str, object]:
        return validate_typed_document(copy.deepcopy(document))

    def test_accepts_semantically_typed_conversion(self) -> None:
        validated = self.validate(self.document)
        self.assertEqual(validated["schema"], "arukellt-verified-core")

    def test_contract_root_must_be_bool(self) -> None:
        document = copy.deepcopy(self.document)
        expression = document["functions"][0]["contracts"][0]["expression"]
        expression["kind"] = "add"
        expression["type_id"] = 1
        with self.assertRaisesRegex(TypedVerifiedCoreError, "contract must have type bool"):
            self.validate(document)

    def test_arithmetic_cannot_claim_bool_result(self) -> None:
        document = copy.deepcopy(self.document)
        expression = document["functions"][0]["contracts"][0]["expression"]
        expression["kind"] = "add"
        with self.assertRaisesRegex(TypedVerifiedCoreError, "arithmetic must preserve"):
            self.validate(document)

    def test_logical_operator_rejects_integer_operands(self) -> None:
        document = copy.deepcopy(self.document)
        expression = document["functions"][0]["contracts"][0]["expression"]
        expression["kind"] = "and"
        with self.assertRaisesRegex(TypedVerifiedCoreError, "logical operands must be bool"):
            self.validate(document)

    def test_result_type_must_match_function_return(self) -> None:
        document = copy.deepcopy(self.document)
        expression = document["functions"][0]["contracts"][0]["expression"]
        expression["operands"][0] = {
            "id": 100,
            "kind": "result",
            "type_id": 2,
        }
        with self.assertRaisesRegex(TypedVerifiedCoreError, "result type does not match"):
            self.validate(document)

    def test_result_is_rejected_outside_ensures(self) -> None:
        document = copy.deepcopy(self.document)
        contract = document["functions"][0]["contracts"][0]
        contract["kind"] = "requires"
        contract.pop("result_name", None)
        contract["expression"]["operands"][0] = {
            "id": 100,
            "kind": "result",
            "type_id": 1,
        }
        with self.assertRaisesRegex(TypedVerifiedCoreError, "only valid in ensures"):
            self.validate(document)

    def test_signature_parameter_requires_matching_parameter_local(self) -> None:
        document = copy.deepcopy(self.document)
        document["functions"][0]["locals"][0]["storage"] = "local"
        with self.assertRaisesRegex(TypedVerifiedCoreError, "not storage=parameter"):
            self.validate(document)

    def test_parameter_local_cannot_be_absent_from_signature(self) -> None:
        document = copy.deepcopy(self.document)
        document["functions"][0]["signature"]["parameters"] = []
        document["functions"][0]["abi"]["parameters"] = []
        with self.assertRaisesRegex(TypedVerifiedCoreError, "absent from signature"):
            self.validate(document)

    def test_integer_constant_rejects_boolean_payload(self) -> None:
        document = copy.deepcopy(self.document)
        terminator = document["functions"][0]["body"]["blocks"][0]["terminator"]
        terminator["value"] = {
            "kind": "constant",
            "type_id": 1,
            "value": True,
        }
        with self.assertRaisesRegex(TypedVerifiedCoreError, "integer constant requires"):
            self.validate(document)

    def test_expression_ids_are_unique_across_contracts(self) -> None:
        document = copy.deepcopy(self.document)
        duplicate = copy.deepcopy(document["functions"][0]["contracts"][0])
        document["functions"][0]["contracts"].append(duplicate)
        with self.assertRaisesRegex(TypedVerifiedCoreError, "duplicate expression id"):
            self.validate(document)


if __name__ == "__main__":
    unittest.main()
