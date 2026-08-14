from __future__ import annotations
import copy, sys, unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path: sys.path.insert(0, str(SCRIPTS))
from proof.typed_corehir_v2_convert import convert_document
from proof.smtlib_typed_v5 import generate_typed_smtlib


def document():
    rep0 = {"wasm": [], "nullable": False, "size_bytes": 0, "align_bytes": 1}
    return {
        "schema": "arukellt-typed-corehir", "schema_version": 2, "module": "p5-source",
        "target_profile": {"integer_model": "mathematical", "overflow": "checked", "floating_point": "unsupported", "pointer_width": 32},
        "types": [
            {"id": 0, "kind": "unit", "name": "()", "representation": rep0},
            {"id": 1, "kind": "integer", "name": "i32", "bits": 32, "signed": True, "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
            {"id": 2, "kind": "bool", "name": "bool", "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
            {"id": 10, "kind": "enum", "name": "OptionI32", "variants": [{"name": "None", "discriminant": 0, "payload_type_ids": []}, {"name": "Some", "discriminant": 1, "payload_type_ids": [1]}], "representation": rep0},
        ],
        "functions": [{
            "id": 0, "name": "make_some",
            "signature": {"parameters": [{"name": "x", "type_id": 1}], "return_type_id": 10},
            "abi": {"calling_convention": "arukellt", "parameters": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}], "results": [{"type_id": 10, "passing": "value", "wasm": []}]},
            "locals": [{"id": 0, "name": "x", "type_id": 1, "storage": "parameter"}],
            "contracts": [{"kind": "ensures", "result_name": "result", "expression_id": 6}],
            "body": {"root_expression_id": 4, "expressions": [
                {"id": 0, "kind": "local", "type_id": 1, "local_id": 0, "children": []},
                {"id": 1, "kind": "construct", "type_id": 10, "variant_index": 1, "children": [0]},
                {"id": 2, "kind": "block", "type_id": 10, "children": [1]},
                {"id": 3, "kind": "constant", "type_id": 1, "value": 0, "children": []},
                {"id": 4, "kind": "block", "type_id": 10, "children": [1]},
                {"id": 5, "kind": "result", "type_id": 10, "children": []},
                {"id": 6, "kind": "is_variant", "type_id": 2, "variant_index": 1, "children": [5]},
            ]},
        }],
    }

class TypedCoreHIRV2AggregateTests(unittest.TestCase):
    def test_option_constructor_lowers_to_phase5_verified_core(self):
        converted = convert_document(document())
        self.assertEqual(converted["generator"], "arukellt-typed-corehir-converter-v5")
        instruction = converted["functions"][0]["body"]["blocks"][0]["instructions"][0]
        self.assertEqual((instruction["op"], instruction["variant_index"]), ("construct", 1))
        self.assertIn("aggregate-encoding=arukellt-smt-datatype-v1", generate_typed_smtlib(converted))
    def test_bad_variant_is_rejected(self):
        value = document(); value["functions"][0]["body"]["expressions"][1]["variant_index"] = 9
        with self.assertRaisesRegex(ValueError, "variant"):
            convert_document(value)

if __name__ == "__main__": unittest.main()
