from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_typed_v1 import generate_typed_smtlib  # noqa: E402
from proof.verified_core_typed import TypedVerifiedCoreError, validate_typed_document  # noqa: E402


def _types() -> list[dict]:
    return [
        {"id": 0, "kind": "unit", "name": "()", "representation": {"wasm": [], "nullable": False, "size_bytes": 0, "align_bytes": 1}},
        {"id": 1, "kind": "integer", "name": "i32", "bits": 32, "signed": True, "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
        {"id": 2, "kind": "bool", "name": "bool", "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
    ]


def _document(module: str, functions: list[dict]) -> dict:
    return {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": "phase-test",
        "module": module,
        "target_profile": {"integer_model": "mathematical", "overflow": "checked", "floating_point": "unsupported", "pointer_width": 32},
        "types": _types(),
        "functions": functions,
    }


def phase1_document() -> dict:
    function = {
        "id": 0,
        "name": "inc",
        "signature": {"parameters": [{"name": "x", "type_id": 1}], "return_type_id": 1},
        "abi": {"calling_convention": "arukellt", "parameters": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}], "results": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}]},
        "locals": [{"id": 0, "name": "x", "type_id": 1, "storage": "parameter"}, {"id": 1, "name": "tmp", "type_id": 1, "storage": "temporary"}],
        "contracts": [{"kind": "ensures", "result_name": "result", "expression": {"id": 0, "kind": "gt", "type_id": 2, "operands": [{"id": 1, "kind": "result", "type_id": 1}, {"id": 2, "kind": "local", "type_id": 1, "local_id": 0}]}}],
        "body": {"entry_block": 0, "blocks": [{"id": 0, "parameters": [], "instructions": [{"id": 0, "op": "binary", "operator": "add", "dest_local_id": 1, "type_id": 1, "arguments": [{"kind": "local", "type_id": 1, "local_id": 0}, {"kind": "constant", "type_id": 1, "value": 1}]}], "terminator": {"kind": "return", "value": {"kind": "local", "type_id": 1, "local_id": 1}}}]},
    }
    return _document("phase1", [function])


def phase2_document() -> dict:
    function = {
        "id": 0,
        "name": "max",
        "signature": {"parameters": [{"name": "a", "type_id": 1}, {"name": "b", "type_id": 1}], "return_type_id": 1},
        "abi": {"calling_convention": "arukellt", "parameters": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}, {"type_id": 1, "passing": "value", "wasm": ["i32"]}], "results": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}]},
        "locals": [
            {"id": 0, "name": "a", "type_id": 1, "storage": "parameter"},
            {"id": 1, "name": "b", "type_id": 1, "storage": "parameter"},
            {"id": 2, "name": "cond", "type_id": 2, "storage": "temporary"},
            {"id": 3, "name": "selected", "type_id": 1, "storage": "block-parameter"},
        ],
        "contracts": [
            {"kind": "ensures", "result_name": "result", "expression": {"id": 0, "kind": "ge", "type_id": 2, "operands": [{"id": 1, "kind": "result", "type_id": 1}, {"id": 2, "kind": "local", "type_id": 1, "local_id": 0}]}},
            {"kind": "ensures", "result_name": "result", "expression": {"id": 3, "kind": "ge", "type_id": 2, "operands": [{"id": 4, "kind": "result", "type_id": 1}, {"id": 5, "kind": "local", "type_id": 1, "local_id": 1}]}},
        ],
        "body": {
            "entry_block": 0,
            "blocks": [
                {"id": 0, "parameters": [], "instructions": [{"id": 0, "op": "binary", "operator": "ge", "dest_local_id": 2, "type_id": 2, "arguments": [{"kind": "local", "type_id": 1, "local_id": 0}, {"kind": "local", "type_id": 1, "local_id": 1}]}], "terminator": {"kind": "branch", "condition": {"kind": "local", "type_id": 2, "local_id": 2}, "then_target": 1, "else_target": 2, "then_arguments": [], "else_arguments": []}},
                {"id": 1, "parameters": [], "instructions": [], "terminator": {"kind": "goto", "target": 3, "arguments": [{"kind": "local", "type_id": 1, "local_id": 0}]}},
                {"id": 2, "parameters": [], "instructions": [], "terminator": {"kind": "goto", "target": 3, "arguments": [{"kind": "local", "type_id": 1, "local_id": 1}]}},
                {"id": 3, "parameters": [{"local_id": 3, "type_id": 1}], "instructions": [], "terminator": {"kind": "return", "value": {"kind": "local", "type_id": 1, "local_id": 3}}},
            ],
        },
    }
    return _document("phase2", [function])


class ProofProgramPhaseTests(unittest.TestCase):
    def test_phase1_straight_line_instruction_generates_vc(self) -> None:
        rendered = generate_typed_smtlib(phase1_document())
        self.assertIn("(set-logic QF_NIA)", rendered)
        self.assertIn("(+ f0_inc_arg_0_x 1)", rendered)

    def test_phase1_rejects_binary_type_mismatch(self) -> None:
        document = phase1_document()
        document["functions"][0]["body"]["blocks"][0]["instructions"][0]["arguments"][1] = {"kind": "constant", "type_id": 2, "value": True}
        with self.assertRaises(TypedVerifiedCoreError):
            validate_typed_document(document)

    def test_phase1_emits_division_side_condition(self) -> None:
        document = phase1_document()
        document["functions"][0]["body"]["blocks"][0]["instructions"][0]["operator"] = "div"
        self.assertIn("(distinct 1 0)", generate_typed_smtlib(document))

    def test_phase2_acyclic_cfg_uses_path_conditions_and_block_parameter(self) -> None:
        rendered = generate_typed_smtlib(phase2_document())
        self.assertGreaterEqual(rendered.count("(check-sat)"), 4)
        self.assertIn("(not (>= f0_max_arg_0_a f0_max_arg_1_b))", rendered)

    def test_phase2_rejects_cycle(self) -> None:
        document = phase2_document()
        document["functions"][0]["body"]["blocks"][3]["terminator"] = {"kind": "goto", "target": 0, "arguments": []}
        with self.assertRaisesRegex(TypedVerifiedCoreError, "cyclic CFG"):
            validate_typed_document(document)


if __name__ == "__main__":
    unittest.main()
