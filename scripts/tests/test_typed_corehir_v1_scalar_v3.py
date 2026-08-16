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

from proof.smtlib_typed_v7 import generate_typed_smtlib
from proof.typed_corehir_v1_scalar_v3 import GENERATOR, upgrade_scalar_document
from proof.typed_corehir_v3 import validate_document
from proof.typed_corehir_v3_convert import CONVERTER, convert_document


def source_document() -> dict:
    value = json.loads((ROOT / "tests" / "proof" / "typed-corehir.json").read_text(encoding="utf-8"))
    value["generator"] = "arukellt-selfhost"
    function = value["functions"][0]
    expressions = function["body"]["expressions"]
    expressions.append(copy.deepcopy(expressions[1]))
    expressions[-1]["id"] = 3
    function["contracts"][0]["expression_id"] = 2
    expressions[2]["children"] = [1, 3]
    return value


def vec_source_document() -> dict:
    value = source_document()
    value["module"] = "vec-read"
    value["types"].append({
        "id": 101,
        "kind": "reference",
        "name": "display-name-is-not-semantic",
        "value_type": "gc-ref",
        "representation": {
            "kind": "gc-ref",
            "wasm": ["gc-ref"],
            "nullable": True,
            "size_bytes": 4,
            "align_bytes": 4,
        },
    })
    function = value["functions"][0]
    function["name"] = "array_at"
    function["signature"] = {
        "parameters": [
            {"name": "a", "type_id": 101},
            {"name": "i", "type_id": 1},
        ],
        "return_type_id": 1,
    }
    function["abi"] = {
        "calling_convention": "arukellt",
        "parameters": [
            {"type_id": 101, "passing": "reference", "wasm": ["gc-ref"]},
            {"type_id": 1, "passing": "value", "wasm": ["i32"]},
        ],
        "results": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}],
    }
    function["locals"] = [
        {"id": 0, "name": "a", "type_id": 101, "storage": "parameter"},
        {"id": 1, "name": "i", "type_id": 1, "storage": "parameter"},
    ]
    function["contracts"] = [
        {"kind": "requires", "expression_id": 8},
        {"kind": "ensures", "expression_id": 13, "result_name": "result"},
    ]
    function["body"] = {
        "root_expr_id": 14,
        "expressions": [
            {"id": 0, "kind": "ident", "kind_id": 5, "type_id": 101, "value_type": "gc-ref", "text": "a", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": []},
            {"id": 1, "kind": "ident", "kind_id": 5, "type_id": 1, "value_type": "i32", "text": "i", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": []},
            {"id": 2, "kind": "index", "kind_id": 16, "type_id": 1, "value_type": "i32", "text": "", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": [0, 1]},
            {"id": 3, "kind": "ident", "kind_id": 5, "type_id": 1, "value_type": "i32", "text": "i", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": []},
            {"id": 4, "kind": "int", "kind_id": 6, "type_id": 1, "value_type": "i32", "text": "0", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": []},
            {"id": 5, "kind": "binary", "kind_id": 11, "type_id": 2, "value_type": "i32", "text": ">=", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": [3, 4]},
            {"id": 6, "kind": "method-call", "kind_id": 19, "type_id": 1, "value_type": "i32", "text": "len", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": [0]},
            {"id": 7, "kind": "binary", "kind_id": 11, "type_id": 2, "value_type": "i32", "text": "<", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": [1, 6]},
            {"id": 8, "kind": "binary", "kind_id": 11, "type_id": 2, "value_type": "i32", "text": "&&", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": [5, 7]},
            {"id": 9, "kind": "ident", "kind_id": 5, "type_id": 1, "value_type": "i32", "text": "result", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": []},
            {"id": 10, "kind": "ident", "kind_id": 5, "type_id": 101, "value_type": "gc-ref", "text": "a", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": []},
            {"id": 11, "kind": "ident", "kind_id": 5, "type_id": 1, "value_type": "i32", "text": "i", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": []},
            {"id": 12, "kind": "index", "kind_id": 16, "type_id": 1, "value_type": "i32", "text": "", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": [10, 11]},
            {"id": 13, "kind": "binary", "kind_id": 11, "type_id": 2, "value_type": "i32", "text": "==", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": [9, 12]},
            {"id": 14, "kind": "block", "kind_id": 1, "type_id": 1, "value_type": "i32", "text": "", "int_value": 0, "float_value": 0.0, "span_start": 0, "children": [2, 8, 13]},
        ],
    }
    return value


class TypedCoreHIRV1ScalarV3Tests(unittest.TestCase):
    def test_scalar_selfhost_source_upgrades_and_reaches_phase7_smt(self):
        upgraded = upgrade_scalar_document(source_document())
        self.assertEqual(upgraded["schema_version"], 3)
        self.assertEqual(upgraded["generator"], GENERATOR)
        self.assertEqual(upgraded["target_profile"]["integer_model"], "machine")
        self.assertEqual(upgraded["target_profile"]["overflow"], "checked")
        self.assertEqual(upgraded["target_profile"]["floating_point"], "unsupported")
        self.assertEqual(upgraded["proof_memory"], {"model": "arukellt-readonly-heap-v1", "references": []})
        self.assertNotIn(2, upgraded["functions"][0]["body"]["expressions"][0]["children"])
        validate_document(upgraded)

        converted = convert_document(upgraded)
        self.assertEqual(converted["generator"], CONVERTER)
        rendered = generate_typed_smtlib(converted)
        self.assertIn("(check-sat)", rendered)
        self.assertIn("machine", rendered)

    def test_structural_vec_read_upgrades_to_phase7_memory_semantics(self):
        upgraded = upgrade_scalar_document(vec_source_document())
        self.assertEqual(upgraded["generator"], GENERATOR)
        reference = next(entry for entry in upgraded["types"] if entry["id"] == 101)
        self.assertEqual(reference["kind"], "reference")
        self.assertEqual(reference["pointee_type_id"], 1)
        self.assertEqual(upgraded["proof_memory"]["references"], [{
            "type_id": 101,
            "kind": "array",
            "element_type_id": 1,
            "length_type_id": 1,
        }])
        expressions = upgraded["functions"][0]["body"]["expressions"]
        self.assertTrue(any(expression["kind"] == "array_get" for expression in expressions))
        self.assertTrue(any(expression["kind"] == "array_len" for expression in expressions))
        converted = convert_document(upgraded)
        rendered = generate_typed_smtlib(converted)
        self.assertIn("arukellt_heap_t101_get", rendered)
        self.assertIn("arukellt_heap_t101_len", rendered)
        self.assertIn("readonly-memory-model=arukellt-readonly-heap-v1", rendered)

    def test_structural_vec_display_name_is_non_semantic(self):
        left = upgrade_scalar_document(vec_source_document())
        right_source = vec_source_document()
        next(entry for entry in right_source["types"] if entry["id"] == 101)["name"] = "renamed-display-only"
        right = upgrade_scalar_document(right_source)
        self.assertEqual(
            generate_typed_smtlib(convert_document(left)),
            generate_typed_smtlib(convert_document(right)),
        )

    def test_unknown_reference_typeid_is_rejected(self):
        source = vec_source_document()
        reference = next(entry for entry in source["types"] if entry["id"] == 101)
        reference["id"] = 109
        for function in source["functions"]:
            for parameter in function["signature"]["parameters"]:
                if parameter["type_id"] == 101:
                    parameter["type_id"] = 109
            for item in function["abi"]["parameters"]:
                if item["type_id"] == 101:
                    item["type_id"] = 109
            for local in function["locals"]:
                if local["type_id"] == 101:
                    local["type_id"] = 109
            for expression in function["body"]["expressions"]:
                if expression["type_id"] == 101:
                    expression["type_id"] = 109
        with self.assertRaisesRegex(ValueError, "not a compiler structural Vec TypeId"):
            upgrade_scalar_document(source)

    def test_overflow_capable_source_arithmetic_is_rejected(self):
        source = source_document()
        expression = source["functions"][0]["body"]["expressions"][2]
        expression["text"] = "+"
        expression["type_id"] = 1
        with self.assertRaisesRegex(ValueError, "overflow-capable"):
            upgrade_scalar_document(source)

    def test_used_float_type_is_rejected(self):
        source = source_document()
        source["types"].append({
            "id": 5,
            "kind": "float",
            "name": "f32",
            "value_type": "f32",
            "representation": {"kind": "scalar", "wasm": ["f32"], "nullable": False, "size_bytes": 4, "align_bytes": 4},
        })
        function = source["functions"][0]
        function["signature"]["return_type_id"] = 5
        function["abi"]["results"] = [{"type_id": 5, "passing": "value", "wasm": ["f32"]}]
        function["body"]["expressions"][0]["type_id"] = 5
        function["body"]["expressions"][1]["type_id"] = 5
        with self.assertRaisesRegex(ValueError, "proof bridge rejects"):
            upgrade_scalar_document(source)

    def test_selfhost_char_typeid_is_not_reinterpreted_as_signed_integer(self):
        source = source_document()
        source["types"].append({
            "id": 3,
            "kind": "integer",
            "name": "char",
            "bits": 32,
            "signed": True,
            "value_type": "i32",
            "representation": {"kind": "scalar", "wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4},
        })
        function = source["functions"][0]
        function["signature"]["return_type_id"] = 3
        function["abi"]["results"] = [{"type_id": 3, "passing": "value", "wasm": ["i32"]}]
        function["body"]["expressions"][0]["type_id"] = 3
        function["body"]["expressions"][1]["type_id"] = 3
        with self.assertRaisesRegex(ValueError, "selfhost TypeId 3"):
            upgrade_scalar_document(source)


if __name__ == "__main__":
    unittest.main()
