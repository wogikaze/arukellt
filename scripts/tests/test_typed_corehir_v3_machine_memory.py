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
from proof.typed_corehir_v3 import validate_document
from proof.typed_corehir_v3_convert import convert_document


def _rep(wasm, size, nullable=False):
    return {"wasm": wasm, "nullable": nullable, "size_bytes": size, "align_bytes": max(1, size)}


def _abi(type_id, wasm):
    return {"type_id": type_id, "passing": "reference" if wasm == "gc-ref" else "value", "wasm": [wasm]}


def _expr(expr_id, kind, type_id, children=None, **extra):
    result = {"id": expr_id, "kind": kind, "type_id": type_id, "children": [] if children is None else children}
    result.update(extra)
    return result


def source_document():
    expressions = [
        _expr(0, "local", 4, local_id=0),
        _expr(1, "local", 1, local_id=1),
        _expr(2, "array_get", 1, [0, 1]),
        _expr(10, "local", 1, local_id=1),
        _expr(11, "constant", 1, value=0),
        _expr(12, "ge", 2, [10, 11]),
        _expr(13, "local", 4, local_id=0),
        _expr(14, "array_len", 1, [13]),
        _expr(15, "local", 1, local_id=1),
        _expr(16, "lt", 2, [15, 14]),
        _expr(17, "and", 2, [12, 16]),
        _expr(20, "result", 1),
        _expr(21, "local", 4, local_id=0),
        _expr(22, "local", 1, local_id=1),
        _expr(23, "array_get", 1, [21, 22]),
        _expr(24, "eq", 2, [20, 23]),
    ]
    return {
        "schema": "arukellt-typed-corehir",
        "schema_version": 3,
        "generator": "phase7-source-test",
        "capability_profile": "proof-phases-0-7",
        "module": "phase7_source",
        "target_profile": {
            "integer_model": "machine",
            "overflow": "checked",
            "floating_point": "unsupported",
            "pointer_width": 32,
        },
        "types": [
            {"id": 0, "kind": "unit", "name": "()", "representation": _rep([], 0)},
            {"id": 1, "kind": "integer", "name": "Index", "bits": 32, "signed": True, "representation": _rep(["i32"], 4)},
            {"id": 2, "kind": "bool", "name": "bool", "representation": _rep(["i32"], 4)},
            {"id": 3, "kind": "struct", "name": "UnusedPureAggregate", "fields": [{"name": "value", "type_id": 1}], "representation": _rep(["gc-ref"], 4)},
            {"id": 4, "kind": "reference", "name": "IndexArray", "pointee_type_id": 1, "representation": _rep(["gc-ref"], 4, nullable=True)},
        ],
        "proof_memory": {
            "model": "arukellt-readonly-heap-v1",
            "references": [{"type_id": 4, "kind": "array", "element_type_id": 1, "length_type_id": 1}],
        },
        "functions": [
            {
                "id": 0,
                "name": "array_at",
                "signature": {"parameters": [{"name": "a", "type_id": 4}, {"name": "i", "type_id": 1}], "return_type_id": 1},
                "abi": {"calling_convention": "arukellt", "parameters": [_abi(4, "gc-ref"), _abi(1, "i32")], "results": [_abi(1, "i32")]},
                "locals": [
                    {"id": 0, "name": "a", "type_id": 4, "storage": "parameter"},
                    {"id": 1, "name": "i", "type_id": 1, "storage": "parameter"},
                ],
                "contracts": [
                    {"kind": "requires", "expression_id": 17},
                    {"kind": "ensures", "expression_id": 24, "result_name": "result"},
                ],
                "body": {"root_expression_id": 2, "expressions": expressions},
            }
        ],
    }


class TypedCoreHIRV3MachineMemoryTests(unittest.TestCase):
    def test_schema_declares_versioned_machine_memory_profile(self):
        schema = json.loads((ROOT / "schemas" / "typed-corehir-v3.schema.json").read_text(encoding="utf-8"))
        self.assertEqual(schema["properties"]["schema_version"]["const"], 3)
        self.assertEqual(schema["properties"]["capability_profile"]["const"], "proof-phases-0-7")
        self.assertEqual(schema["properties"]["proof_memory"]["$ref"], "#/$defs/proof_memory")

    def test_source_v3_lowers_to_readonly_verified_core_and_smt(self):
        source = source_document()
        validate_document(source)
        converted = convert_document(source)
        self.assertEqual(converted["generator"], "arukellt-typed-corehir-converter-v7")
        self.assertEqual(converted["proof_memory"]["model"], "arukellt-readonly-heap-v1")
        instruction = converted["functions"][0]["body"]["blocks"][0]["instructions"][0]
        self.assertEqual(instruction["op"], "array_get")
        rendered = generate_typed_smtlib(converted)
        self.assertIn("arukellt_heap_t4_get", rendered)
        self.assertIn("arukellt_heap_t4_len", rendered)
        self.assertIn("readonly-memory-model=arukellt-readonly-heap-v1", rendered)

    def test_reference_display_name_does_not_change_source_to_smt_encoding(self):
        left = source_document()
        right = copy.deepcopy(left)
        right["types"][4]["name"] = "DisplayOnlyRename"
        self.assertEqual(generate_typed_smtlib(convert_document(left)), generate_typed_smtlib(convert_document(right)))

    def test_symbolic_shift_is_rejected_at_source_boundary(self):
        source = source_document()
        function = source["functions"][0]
        expressions = function["body"]["expressions"]
        expressions.extend([
            _expr(30, "local", 1, local_id=1),
            _expr(31, "local", 1, local_id=1),
            _expr(32, "shl", 1, [30, 31]),
        ])
        function["body"]["root_expression_id"] = 32
        with self.assertRaisesRegex(ValueError, "shift count must be immediate"):
            validate_document(source)

    def test_reference_descriptor_set_must_match_source_types(self):
        source = source_document()
        source["proof_memory"]["references"] = []
        with self.assertRaisesRegex(ValueError, "exactly all reference TypeIds"):
            validate_document(source)


if __name__ == "__main__":
    unittest.main()
