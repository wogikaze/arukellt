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

from proof.typed_corehir_typed_convert import ExplicitTypedCoreHirError, convert_typed_document


class TypedCoreHirConvertTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = json.loads((ROOT / "tests" / "proof" / "typed-corehir.json").read_text())

    def test_identity_converts_through_v3_program_lowering(self) -> None:
        converted = convert_typed_document(copy.deepcopy(self.source))
        function = converted["functions"][0]
        self.assertEqual(converted["schema"], "arukellt-verified-core")
        self.assertEqual(converted["generator"], "arukellt-typed-corehir-converter-v3")
        self.assertEqual(function["body"]["blocks"][0]["terminator"]["value"], {"kind": "local", "type_id": 1, "local_id": 0})
        self.assertEqual(function["contracts"][0]["expression"]["kind"], "ge")

    def test_integer_metadata_remains_explicit(self) -> None:
        converted = convert_typed_document(copy.deepcopy(self.source))
        integer = next(entry for entry in converted["types"] if entry["id"] == 1)
        self.assertEqual(integer["name"], "ApplicationCounter")
        self.assertEqual(integer["bits"], 32)
        self.assertIs(integer["signed"], True)
        self.assertEqual(integer["representation"]["wasm"], ["i32"])

    def test_result_identifier_converts_to_result_node(self) -> None:
        document = copy.deepcopy(self.source)
        function = document["functions"][0]
        function["body"]["expressions"].append({"id": 3, "kind": "ident", "kind_id": 5, "type_id": 1, "value_type": "i32", "text": "result", "int_value": 0, "float_value": 0.0, "span_start": 3, "children": []})
        function["body"]["expressions"][2]["children"] = [3, 1]
        converted = convert_typed_document(document)
        self.assertEqual(converted["functions"][0]["contracts"][0]["expression"]["operands"][0]["kind"], "result")

    def test_missing_integer_metadata_fails_closed(self) -> None:
        document = copy.deepcopy(self.source)
        document["types"][1].pop("bits")
        with self.assertRaisesRegex(ExplicitTypedCoreHirError, "expected 32 or 64"):
            convert_typed_document(document)

    def test_integer_representation_mismatch_fails_closed(self) -> None:
        document = copy.deepcopy(self.source)
        document["types"][1]["representation"]["wasm"] = ["i64"]
        with self.assertRaisesRegex(ExplicitTypedCoreHirError, "representation mismatch"):
            convert_typed_document(document)

    def test_straight_line_block_accepts_multiple_expressions(self) -> None:
        document = copy.deepcopy(self.source)
        function = document["functions"][0]
        function["body"]["expressions"][0]["children"] = [1, 1, 2]
        converted = convert_typed_document(document)
        self.assertEqual(converted["functions"][0]["body"]["blocks"][0]["terminator"]["kind"], "return")

    def test_unknown_contract_identifier_fails_closed(self) -> None:
        document = copy.deepcopy(self.source)
        document["functions"][0]["body"]["expressions"][1]["text"] = "missing"
        with self.assertRaisesRegex(ExplicitTypedCoreHirError, "unknown proof identifier"):
            convert_typed_document(document)

    def test_unreachable_reference_type_is_excluded(self) -> None:
        document = copy.deepcopy(self.source)
        document["types"].append({"id": 3, "kind": "reference", "name": "String", "value_type": "gc-ref", "representation": {"kind": "gc-ref", "wasm": ["gc-ref"], "nullable": True, "size_bytes": 4, "align_bytes": 4}})
        converted = convert_typed_document(document)
        self.assertNotIn(3, {entry["id"] for entry in converted["types"]})


if __name__ == "__main__":
    unittest.main()
