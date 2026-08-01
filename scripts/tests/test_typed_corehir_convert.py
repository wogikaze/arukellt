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

from proof.typed_corehir_convert import UnsupportedTypedCoreHir, convert_document  # noqa: E402


class TypedCoreHirConvertTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = json.loads((ROOT / "tests" / "proof" / "typed-corehir.json").read_text())

    def test_identity_converts_to_structured_verified_core(self) -> None:
        converted = convert_document(copy.deepcopy(self.source))
        function = converted["functions"][0]
        self.assertEqual(converted["schema"], "arukellt-verified-core")
        self.assertEqual(function["body"]["blocks"][0]["terminator"]["value"], {
            "kind": "local",
            "type_id": 1,
            "local_id": 0,
        })
        expression = function["contracts"][0]["expression"]
        self.assertEqual(expression["kind"], "ge")
        self.assertEqual(expression["operands"][0]["kind"], "local")
        self.assertEqual(expression["operands"][1]["kind"], "local")

    def test_result_identifier_converts_to_result_node(self) -> None:
        document = copy.deepcopy(self.source)
        function = document["functions"][0]
        function["body"]["expressions"].append({
            "id": 3,
            "kind": "ident",
            "kind_id": 5,
            "type_id": 1,
            "value_type": "i32",
            "text": "result",
            "int_value": 0,
            "float_value": 0.0,
            "span_start": 3,
            "children": [],
        })
        function["body"]["expressions"][2]["children"] = [3, 1]
        converted = convert_document(document)
        expression = converted["functions"][0]["contracts"][0]["expression"]
        self.assertEqual(expression["operands"][0]["kind"], "result")

    def test_multiple_executable_body_children_fail_closed(self) -> None:
        document = copy.deepcopy(self.source)
        function = document["functions"][0]
        function["body"]["root_expr_id"] = 0
        function["body"]["expressions"][0]["children"] = [1, 1, 2]
        with self.assertRaisesRegex(UnsupportedTypedCoreHir, "exactly one non-contract return"):
            convert_document(document)

    def test_unknown_contract_identifier_fails_closed(self) -> None:
        document = copy.deepcopy(self.source)
        function = document["functions"][0]
        function["body"]["expressions"][1]["text"] = "missing"
        with self.assertRaisesRegex(UnsupportedTypedCoreHir, "unknown proof identifier"):
            convert_document(document)

    def test_uncontracted_function_and_unused_reference_type_are_ignored(self) -> None:
        document = copy.deepcopy(self.source)
        document["types"].append({
            "id": 3,
            "kind": "reference",
            "name": "String",
            "value_type": "gc-ref",
            "representation": {
                "kind": "gc-ref",
                "wasm": ["gc-ref"],
                "nullable": True,
                "size_bytes": 4,
                "align_bytes": 4,
            },
        })
        helper = copy.deepcopy(document["functions"][0])
        helper["id"] = 1
        helper["name"] = "runtime_helper"
        helper["contracts"] = []
        helper["signature"]["parameters"][0]["type_id"] = 3
        helper["signature"]["return_type_id"] = 3
        helper["locals"][0]["type_id"] = 3
        for expression in helper["body"]["expressions"]:
            expression["type_id"] = 3
            expression["value_type"] = "gc-ref"
        document["functions"].append(helper)

        converted = convert_document(document)
        self.assertEqual([function["name"] for function in converted["functions"]], ["identity"])
        self.assertNotIn(3, [entry["id"] for entry in converted["types"]])

    def test_reachable_reference_type_fails_closed(self) -> None:
        document = copy.deepcopy(self.source)
        document["types"].append({
            "id": 3,
            "kind": "reference",
            "name": "String",
            "value_type": "gc-ref",
            "representation": {
                "kind": "gc-ref",
                "wasm": ["gc-ref"],
                "nullable": True,
                "size_bytes": 4,
                "align_bytes": 4,
            },
        })
        function = document["functions"][0]
        function["signature"]["parameters"][0]["type_id"] = 3
        function["locals"][0]["type_id"] = 3
        function["body"]["expressions"][1]["type_id"] = 3
        function["body"]["expressions"][1]["value_type"] = "gc-ref"
        with self.assertRaisesRegex(UnsupportedTypedCoreHir, "unsupported reachable proof type"):
            convert_document(document)


if __name__ == "__main__":
    unittest.main()
