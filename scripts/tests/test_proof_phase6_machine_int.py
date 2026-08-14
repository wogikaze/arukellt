from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.machine_integer_semantics import validate_machine_integer_semantics
from proof.smtlib_typed_v6 import generate_typed_smtlib


def _rep(wasm, size):
    return {"wasm": wasm, "nullable": False, "size_bytes": size, "align_bytes": max(1, size)}


def _types():
    return [
        {"id": 0, "kind": "unit", "name": "()", "representation": _rep([], 0)},
        {"id": 1, "kind": "integer", "name": "Counter32", "bits": 32, "signed": True, "representation": _rep(["i32"], 4)},
        {"id": 2, "kind": "bool", "name": "bool", "representation": _rep(["i32"], 4)},
        {"id": 3, "kind": "integer", "name": "Counter64", "bits": 64, "signed": True, "representation": _rep(["i64"], 8)},
    ]


def _abi(type_id, wasm):
    return {"type_id": type_id, "passing": "value", "wasm": [wasm]}


def _value(local_id, type_id):
    return {"kind": "local", "local_id": local_id, "type_id": type_id}


def _constant(value, type_id):
    return {"kind": "constant", "value": value, "type_id": type_id}


def _leaf(expr_id, kind, type_id, **extra):
    return {"id": expr_id, "kind": kind, "type_id": type_id, **extra}


def _base_document():
    requires = {
        "id": 2,
        "kind": "le",
        "type_id": 2,
        "operands": [
            _leaf(0, "local", 1, local_id=0),
            _leaf(1, "constant", 1, value=2147483646),
        ],
    }
    ensures = {
        "id": 5,
        "kind": "gt",
        "type_id": 2,
        "operands": [
            _leaf(3, "result", 1),
            _leaf(4, "local", 1, local_id=0),
        ],
    }
    return {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": "phase6-test",
        "module": "phase6",
        "target_profile": {
            "integer_model": "machine",
            "overflow": "checked",
            "floating_point": "unsupported",
            "pointer_width": 32,
        },
        "types": _types(),
        "functions": [
            {
                "id": 0,
                "name": "checked_inc",
                "signature": {"parameters": [{"name": "x", "type_id": 1}], "return_type_id": 1},
                "abi": {"calling_convention": "arukellt", "parameters": [_abi(1, "i32")], "results": [_abi(1, "i32")]},
                "locals": [
                    {"id": 0, "name": "x", "type_id": 1, "storage": "parameter"},
                    {"id": 1, "name": "$one", "type_id": 1, "storage": "temporary"},
                    {"id": 2, "name": "$sum", "type_id": 1, "storage": "temporary"},
                ],
                "contracts": [
                    {"kind": "requires", "expression": requires},
                    {"kind": "ensures", "result_name": "result", "expression": ensures},
                ],
                "body": {
                    "entry_block": 0,
                    "blocks": [
                        {
                            "id": 0,
                            "parameters": [],
                            "instructions": [
                                {"id": 0, "op": "const", "dest_local_id": 1, "type_id": 1, "value": 1},
                                {
                                    "id": 1,
                                    "op": "binary",
                                    "operator": "add",
                                    "dest_local_id": 2,
                                    "type_id": 1,
                                    "arguments": [_value(0, 1), _value(1, 1)],
                                },
                            ],
                            "terminator": {"kind": "return", "value": _value(2, 1)},
                        }
                    ],
                },
            }
        ],
    }


class ProofPhase6MachineIntegerTests(unittest.TestCase):
    def test_checked_add_emits_parameter_and_result_ranges(self):
        rendered = generate_typed_smtlib(_base_document())
        self.assertIn("machine-int-profile=arukellt-machine-int-v1", rendered)
        self.assertIn("machine-int-encoding=arukellt-machine-int-range-v1", rendered)
        self.assertIn("2147483647", rendered)
        self.assertIn("-2147483648", rendered)
        self.assertIn(".side[0]", rendered)

    def test_type_display_name_does_not_change_machine_encoding(self):
        left = _base_document()
        right = copy.deepcopy(left)
        right["types"][1]["name"] = "RenamedWithoutSemanticMeaning"
        right["types"][3]["name"] = "AnotherName"
        self.assertEqual(generate_typed_smtlib(left), generate_typed_smtlib(right))

    def test_out_of_range_constant_fails_closed(self):
        value = _base_document()
        value["functions"][0]["body"]["blocks"][0]["instructions"][0]["value"] = 2147483648
        with self.assertRaisesRegex(ValueError, "outside signed 32-bit range"):
            validate_machine_integer_semantics(value)

    def test_signed_division_emits_zero_and_min_overflow_guards(self):
        value = _base_document()
        block = value["functions"][0]["body"]["blocks"][0]
        block["instructions"][0]["value"] = -1
        block["instructions"][1]["operator"] = "div"
        rendered = generate_typed_smtlib(value)
        self.assertIn("arukellt_mi_sdiv", rendered)
        self.assertIn("(distinct", rendered)
        self.assertIn("-2147483648", rendered)
        self.assertIn("(- 1)", rendered)

    def test_signed_remainder_uses_truncating_division_definition(self):
        value = _base_document()
        block = value["functions"][0]["body"]["blocks"][0]
        block["instructions"][0]["value"] = -1
        block["instructions"][1]["operator"] = "mod"
        rendered = generate_typed_smtlib(value)
        self.assertIn("arukellt_mi_srem", rendered)
        self.assertIn("(- a (* b (arukellt_mi_sdiv a b)))", rendered)

    def test_shift_count_is_masked_by_machine_width(self):
        value = _base_document()
        block = value["functions"][0]["body"]["blocks"][0]
        block["instructions"][1]["operator"] = "shl"
        block["instructions"][1]["arguments"] = [_value(0, 1), _constant(-1, 1)]
        rendered = generate_typed_smtlib(value)
        # -1 mod 32 == 31.
        self.assertIn("2147483648", rendered)

    def test_checked_i64_to_i32_narrowing_emits_destination_range(self):
        value = _base_document()
        function = value["functions"][0]
        function["signature"] = {"parameters": [{"name": "x", "type_id": 3}], "return_type_id": 1}
        function["abi"] = {"calling_convention": "arukellt", "parameters": [_abi(3, "i64")], "results": [_abi(1, "i32")]}
        function["locals"] = [
            {"id": 0, "name": "x", "type_id": 3, "storage": "parameter"},
            {"id": 1, "name": "$narrow", "type_id": 1, "storage": "temporary"},
        ]
        function["contracts"] = [
            {
                "kind": "ensures",
                "result_name": "result",
                "expression": {
                    "id": 2,
                    "kind": "le",
                    "type_id": 2,
                    "operands": [_leaf(0, "result", 1), _leaf(1, "constant", 1, value=2147483647)],
                },
            }
        ]
        function["body"]["blocks"][0]["instructions"] = [
            {
                "id": 0,
                "op": "convert",
                "conversion": "checked-narrow",
                "dest_local_id": 1,
                "type_id": 1,
                "arguments": [_value(0, 3)],
            }
        ]
        function["body"]["blocks"][0]["terminator"] = {"kind": "return", "value": _value(1, 1)}
        rendered = generate_typed_smtlib(value)
        self.assertIn("2147483647", rendered)
        self.assertIn("-2147483648", rendered)


if __name__ == "__main__":
    unittest.main()
