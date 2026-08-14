from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.readonly_memory_semantics import validate_readonly_memory_semantics
from proof.smtlib_typed_v7 import generate_typed_smtlib
from proof.typed_admission_v7 import validate_typed_document


def _rep(wasm, size, nullable=False):
    return {"wasm": wasm, "nullable": nullable, "size_bytes": size, "align_bytes": max(1, size)}


def _types():
    return [
        {"id": 0, "kind": "unit", "name": "()", "representation": _rep([], 0)},
        {"id": 1, "kind": "integer", "name": "Index", "bits": 32, "signed": True, "representation": _rep(["i32"], 4)},
        {"id": 2, "kind": "bool", "name": "bool", "representation": _rep(["i32"], 4)},
        {
            "id": 3,
            "kind": "struct",
            "name": "Point",
            "fields": [{"name": "x", "type_id": 1}, {"name": "y", "type_id": 1}],
            "representation": _rep(["gc-ref"], 4),
        },
        {
            "id": 4,
            "kind": "reference",
            "name": "PointRef",
            "pointee_type_id": 3,
            "representation": _rep(["gc-ref"], 4, nullable=False),
        },
        {
            "id": 5,
            "kind": "reference",
            "name": "IndexArrayRef",
            "pointee_type_id": 1,
            "representation": _rep(["gc-ref"], 4, nullable=True),
        },
    ]


def _abi(type_id, wasm):
    return {"type_id": type_id, "passing": "reference" if wasm == "gc-ref" else "value", "wasm": [wasm]}


def _local(local_id, type_id):
    return {"kind": "local", "local_id": local_id, "type_id": type_id}


def _constant(value, type_id):
    return {"kind": "constant", "value": value, "type_id": type_id}


def _expr(expr_id, kind, type_id, operands=None, **extra):
    value = {"id": expr_id, "kind": kind, "type_id": type_id}
    if operands is not None:
        value["operands"] = operands
    value.update(extra)
    return value


def _base():
    return {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": "phase7-test",
        "module": "phase7",
        "target_profile": {
            "integer_model": "machine",
            "overflow": "checked",
            "floating_point": "unsupported",
            "pointer_width": 32,
        },
        "types": _types(),
        "proof_memory": {
            "model": "arukellt-readonly-heap-v1",
            "references": [
                {"type_id": 4, "kind": "object", "pointee_type_id": 3},
                {"type_id": 5, "kind": "array", "element_type_id": 1, "length_type_id": 1},
            ],
        },
        "functions": [],
    }


def _object_function():
    field_expr = _expr(1, "load_field", 1, [_expr(0, "local", 4, local_id=0)], field_index=0)
    ensures = _expr(4, "eq", 2, [
        _expr(2, "result", 1),
        _expr(3, "load_field", 1, [_expr(5, "local", 4, local_id=0)], field_index=0),
    ])
    return {
        "id": 0,
        "name": "point_x",
        "signature": {"parameters": [{"name": "p", "type_id": 4}], "return_type_id": 1},
        "abi": {"calling_convention": "arukellt", "parameters": [_abi(4, "gc-ref")], "results": [_abi(1, "i32")]},
        "locals": [
            {"id": 0, "name": "p", "type_id": 4, "storage": "parameter"},
            {"id": 1, "name": "$x", "type_id": 1, "storage": "temporary"},
        ],
        "contracts": [{"kind": "ensures", "result_name": "result", "expression": ensures}],
        "body": {
            "entry_block": 0,
            "blocks": [{
                "id": 0,
                "parameters": [],
                "instructions": [{
                    "id": 0,
                    "op": "load_field",
                    "field_index": 0,
                    "dest_local_id": 1,
                    "type_id": 1,
                    "arguments": [_local(0, 4)],
                }],
                "terminator": {"kind": "return", "value": _local(1, 1)},
            }],
        },
    }


def _array_function():
    requires = _expr(9, "and", 2, [
        _expr(2, "ge", 2, [_expr(0, "local", 1, local_id=1), _expr(1, "constant", 1, value=0)]),
        _expr(8, "lt", 2, [
            _expr(3, "local", 1, local_id=1),
            _expr(7, "array_len", 1, [_expr(6, "local", 5, local_id=0)]),
        ]),
    ])
    ensures = _expr(14, "eq", 2, [
        _expr(10, "result", 1),
        _expr(13, "array_get", 1, [
            _expr(11, "local", 5, local_id=0),
            _expr(12, "local", 1, local_id=1),
        ]),
    ])
    return {
        "id": 1,
        "name": "array_at",
        "signature": {"parameters": [{"name": "a", "type_id": 5}, {"name": "i", "type_id": 1}], "return_type_id": 1},
        "abi": {"calling_convention": "arukellt", "parameters": [_abi(5, "gc-ref"), _abi(1, "i32")], "results": [_abi(1, "i32")]},
        "locals": [
            {"id": 0, "name": "a", "type_id": 5, "storage": "parameter"},
            {"id": 1, "name": "i", "type_id": 1, "storage": "parameter"},
            {"id": 2, "name": "$value", "type_id": 1, "storage": "temporary"},
        ],
        "contracts": [
            {"kind": "requires", "expression": requires},
            {"kind": "ensures", "result_name": "result", "expression": ensures},
        ],
        "body": {
            "entry_block": 0,
            "blocks": [{
                "id": 0,
                "parameters": [],
                "instructions": [{
                    "id": 0,
                    "op": "array_get",
                    "dest_local_id": 2,
                    "type_id": 1,
                    "arguments": [_local(0, 5), _local(1, 1)],
                }],
                "terminator": {"kind": "return", "value": _local(2, 1)},
            }],
        },
    }


class ProofPhase7ReadOnlyMemoryTests(unittest.TestCase):
    def test_object_field_read_is_identity_stable_and_nonnull_guarded(self):
        document = _base()
        document["functions"] = [_object_function()]
        rendered = generate_typed_smtlib(document)
        self.assertIn("readonly-memory-model=arukellt-readonly-heap-v1", rendered)
        self.assertIn("declare-sort ArukelltRefT4", rendered)
        self.assertIn("arukellt_heap_t4_f0", rendered)
        self.assertIn("arukellt_null_t4", rendered)
        self.assertNotIn("PointRef", rendered)

    def test_array_get_emits_nonnull_and_bounds_obligations(self):
        document = _base()
        document["functions"] = [_array_function()]
        rendered = generate_typed_smtlib(document)
        self.assertIn("arukellt_heap_t5_len", rendered)
        self.assertIn("arukellt_heap_t5_get", rendered)
        self.assertIn("(>=", rendered)
        self.assertIn("(<", rendered)
        self.assertIn(".side[", rendered)

    def test_reference_display_names_do_not_change_encoding(self):
        left = _base()
        left["functions"] = [_array_function()]
        right = copy.deepcopy(left)
        right["types"][4]["name"] = "RenamedObjectReference"
        right["types"][5]["name"] = "RenamedArrayReference"
        self.assertEqual(generate_typed_smtlib(left), generate_typed_smtlib(right))

    def test_mutation_is_rejected_before_smt(self):
        document = _base()
        function = _object_function()
        function["body"]["blocks"][0]["instructions"][0]["op"] = "store_field"
        document["functions"] = [function]
        with self.assertRaisesRegex(ValueError, "mutation/allocation"):
            validate_readonly_memory_semantics(document)

    def test_array_index_type_mismatch_is_rejected(self):
        document = _base()
        function = _array_function()
        function["body"]["blocks"][0]["instructions"][0]["arguments"][1] = _constant(True, 2)
        document["functions"] = [function]
        with self.assertRaisesRegex(ValueError, "array_get TypeId mismatch"):
            validate_readonly_memory_semantics(document)

    def test_aggregate_contained_reference_is_fail_closed(self):
        document = _base()
        document["types"][3]["fields"].append({"name": "parent", "type_id": 4})
        document["functions"] = [_object_function()]
        with self.assertRaisesRegex(ValueError, "aggregate-contained references"):
            validate_typed_document(document)

    def test_memory_function_call_composition_is_fail_closed(self):
        document = _base()
        function = _object_function()
        function["body"]["blocks"][0]["instructions"].append({
            "id": 1,
            "op": "call",
            "callee_id": 0,
            "callee_interface_sha256": "0" * 64,
            "dest_local_id": 1,
            "type_id": 1,
            "arguments": [_local(0, 4)],
        })
        document["functions"] = [function]
        with self.assertRaisesRegex(ValueError, "memory \+ call"):
            validate_readonly_memory_semantics(document)


if __name__ == "__main__":
    unittest.main()
