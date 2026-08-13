from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_typed_v1 import generate_typed_smtlib
from proof.verified_core_interface import bind_call_interfaces
from proof.verified_core_typed import TypedVerifiedCoreError, validate_typed_document


def _types():
    return [
        {"id": 0, "kind": "unit", "name": "()", "representation": {"wasm": [], "nullable": False, "size_bytes": 0, "align_bytes": 1}},
        {"id": 1, "kind": "integer", "name": "i32", "bits": 32, "signed": True, "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
        {"id": 2, "kind": "bool", "name": "bool", "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
    ]


def _contract(kind, result_name=None):
    left = {"id": 1, "kind": "result" if kind == "ensures" else "local", "type_id": 1}
    if kind == "requires":
        left["local_id"] = 0
    expression = {"id": 0, "kind": "ge", "type_id": 2, "operands": [left, {"id": 2, "kind": "constant", "type_id": 1, "value": 0}]}
    result = {"kind": kind, "expression": expression}
    if result_name:
        result["result_name"] = result_name
    return result


def _function(function_id, name, call=None):
    locals_ = [{"id": 0, "name": "x", "type_id": 1, "storage": "parameter"}]
    instructions = []
    returned = {"kind": "local", "type_id": 1, "local_id": 0}
    if call is not None:
        locals_.append({"id": 1, "name": "y", "type_id": 1, "storage": "temporary"})
        instructions.append({"id": 0, "op": "call", "callee_id": call, "dest_local_id": 1, "type_id": 1, "arguments": [{"kind": "local", "type_id": 1, "local_id": 0}]})
        returned = {"kind": "local", "type_id": 1, "local_id": 1}
    return {
        "id": function_id,
        "name": name,
        "signature": {"parameters": [{"name": "x", "type_id": 1}], "return_type_id": 1},
        "abi": {"calling_convention": "arukellt", "parameters": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}], "results": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}]},
        "locals": locals_,
        "contracts": [_contract("requires"), _contract("ensures", "result")],
        "body": {"entry_block": 0, "blocks": [{"id": 0, "parameters": [], "instructions": instructions, "terminator": {"kind": "return", "value": returned}}]},
    }


def document():
    value = {
        "schema": "arukellt-verified-core", "schema_version": 1, "generator": "phase3-test", "module": "phase3",
        "target_profile": {"integer_model": "mathematical", "overflow": "checked", "floating_point": "unsupported", "pointer_width": 32},
        "types": _types(), "functions": [_function(0, "nonnegative_identity"), _function(1, "forward_nonnegative", 0)],
    }
    return bind_call_interfaces(value)


class ProofPhase3CallTests(unittest.TestCase):
    def test_direct_call_is_verified_modularly(self):
        rendered = generate_typed_smtlib(document())
        self.assertIn("callee-requires", rendered)
        self.assertIn("callee-interface-sha256", rendered)
        self.assertIn("f1_forward_nonnegative_b0_i0_call0", rendered)

    def test_stale_callee_interface_is_rejected(self):
        value = document()
        call = value["functions"][1]["body"]["blocks"][0]["instructions"][0]
        call["callee_interface_sha256"] = "0" * 64
        with self.assertRaisesRegex((TypedVerifiedCoreError, ValueError), "interface"):
            validate_typed_document(value)

    def test_recursive_call_graph_is_rejected(self):
        value = document()
        value["functions"][0] = _function(0, "nonnegative_identity", 0)
        bind_call_interfaces(value)
        with self.assertRaisesRegex(TypedVerifiedCoreError, "recursive proof calls"):
            validate_typed_document(value)


if __name__ == "__main__":
    unittest.main()
