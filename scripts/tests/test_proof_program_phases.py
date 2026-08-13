from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_typed_v1 import generate_typed_smtlib  # noqa: E402
from proof.verified_core_typed import TypedVerifiedCoreError, validate_typed_document  # noqa: E402


def _base_function() -> dict:
    return {
        "id": 0,
        "name": "inc",
        "signature": {"parameters": [{"name": "x", "type_id": 1}], "return_type_id": 1},
        "abi": {
            "calling_convention": "arukellt",
            "parameters": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}],
            "results": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}],
        },
        "locals": [
            {"id": 0, "name": "x", "type_id": 1, "storage": "parameter"},
            {"id": 1, "name": "tmp", "type_id": 1, "storage": "temporary"},
        ],
        "contracts": [
            {
                "kind": "ensures",
                "result_name": "result",
                "expression": {
                    "id": 0,
                    "kind": "gt",
                    "type_id": 2,
                    "operands": [
                        {"id": 1, "kind": "result", "type_id": 1},
                        {"id": 2, "kind": "local", "type_id": 1, "local_id": 0},
                    ],
                },
            }
        ],
        "body": {
            "entry_block": 0,
            "blocks": [
                {
                    "id": 0,
                    "parameters": [],
                    "instructions": [
                        {
                            "id": 0,
                            "op": "binary",
                            "operator": "add",
                            "dest_local_id": 1,
                            "type_id": 1,
                            "arguments": [
                                {"kind": "local", "type_id": 1, "local_id": 0},
                                {"kind": "constant", "type_id": 1, "value": 1},
                            ],
                        }
                    ],
                    "terminator": {"kind": "return", "value": {"kind": "local", "type_id": 1, "local_id": 1}},
                }
            ],
        },
    }


def phase1_document() -> dict:
    return {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": "phase-test",
        "module": "phase1",
        "target_profile": {"integer_model": "mathematical", "overflow": "checked", "floating_point": "unsupported", "pointer_width": 32},
        "types": [
            {"id": 0, "kind": "unit", "name": "()", "representation": {"wasm": [], "nullable": False, "size_bytes": 0, "align_bytes": 1}},
            {"id": 1, "kind": "integer", "name": "i32", "bits": 32, "signed": True, "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
            {"id": 2, "kind": "bool", "name": "bool", "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
        ],
        "functions": [_base_function()],
    }


class ProofProgramPhaseTests(unittest.TestCase):
    def test_phase1_straight_line_instruction_generates_vc(self) -> None:
        rendered = generate_typed_smtlib(phase1_document())
        self.assertIn("(set-logic QF_NIA)", rendered)
        self.assertIn("(+ f0_inc_arg_0_x 1)", rendered)
        self.assertIn("(check-sat)", rendered)

    def test_phase1_rejects_binary_type_mismatch(self) -> None:
        document = phase1_document()
        document["functions"][0]["body"]["blocks"][0]["instructions"][0]["arguments"][1] = {"kind": "constant", "type_id": 2, "value": True}
        with self.assertRaises(TypedVerifiedCoreError):
            validate_typed_document(document)

    def test_phase1_emits_division_side_condition(self) -> None:
        document = phase1_document()
        instruction = document["functions"][0]["body"]["blocks"][0]["instructions"][0]
        instruction["operator"] = "div"
        rendered = generate_typed_smtlib(document)
        self.assertIn("(distinct 1 0)", rendered)


if __name__ == "__main__":
    unittest.main()
