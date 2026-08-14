from __future__ import annotations
import sys, unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path: sys.path.insert(0, str(SCRIPTS))
from proof.smtlib_typed_v1 import generate_typed_smtlib
from proof.verified_core_typed import TypedVerifiedCoreError, validate_typed_document

def expr(i, kind, type_id, operands=None, **extra):
    value = {"id": i, "kind": kind, "type_id": type_id}
    if operands is not None: value["operands"] = operands
    value.update(extra); return value

def document():
    x = {"kind": "local", "type_id": 1, "local_id": 0}; zero = {"kind": "constant", "type_id": 1, "value": 0}
    invariant = expr(10, "ge", 2, [expr(11, "local", 1, local_id=0), expr(12, "constant", 1, value=0)])
    decreases = expr(13, "local", 1, local_id=0)
    requires = {"kind": "requires", "expression": expr(0, "ge", 2, [expr(1, "local", 1, local_id=0), expr(2, "constant", 1, value=0)])}
    ensures = {"kind": "ensures", "result_name": "result", "expression": expr(3, "eq", 2, [expr(4, "result", 1), expr(5, "constant", 1, value=0)])}
    return {
        "schema": "arukellt-verified-core", "schema_version": 1, "generator": "phase4-test", "module": "phase4",
        "target_profile": {"integer_model": "mathematical", "overflow": "checked", "floating_point": "unsupported", "pointer_width": 32},
        "types": [
            {"id": 0, "kind": "unit", "name": "()", "representation": {"wasm": [], "nullable": False, "size_bytes": 0, "align_bytes": 1}},
            {"id": 1, "kind": "integer", "name": "i32", "bits": 32, "signed": True, "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
            {"id": 2, "kind": "bool", "name": "bool", "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
        ],
        "functions": [{
            "id": 0, "name": "countdown", "signature": {"parameters": [{"name": "x", "type_id": 1}], "return_type_id": 1},
            "abi": {"calling_convention": "arukellt", "parameters": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}], "results": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}]},
            "locals": [{"id": 0, "name": "x", "type_id": 1, "storage": "parameter"}, {"id": 1, "name": "$cond", "type_id": 2, "storage": "temporary"}, {"id": 2, "name": "$dec", "type_id": 1, "storage": "temporary"}],
            "contracts": [requires, ensures],
            "body": {"entry_block": 0, "blocks": [
                {"id": 0, "parameters": [], "instructions": [], "terminator": {"kind": "goto", "target": 1, "arguments": []}},
                {"id": 1, "parameters": [], "loop": {"invariants": [invariant], "decreases": decreases}, "instructions": [{"id": 0, "op": "binary", "operator": "gt", "dest_local_id": 1, "type_id": 2, "arguments": [x, zero]}], "terminator": {"kind": "branch", "condition": {"kind": "local", "type_id": 2, "local_id": 1}, "then_target": 2, "else_target": 3, "then_arguments": [], "else_arguments": []}},
                {"id": 2, "parameters": [], "instructions": [{"id": 1, "op": "binary", "operator": "sub", "dest_local_id": 2, "type_id": 1, "arguments": [x, {"kind": "constant", "type_id": 1, "value": 1}]}, {"id": 2, "op": "copy", "dest_local_id": 0, "type_id": 1, "arguments": [{"kind": "local", "type_id": 1, "local_id": 2}]}], "terminator": {"kind": "goto", "target": 1, "arguments": []}},
                {"id": 3, "parameters": [], "instructions": [], "terminator": {"kind": "return", "value": x}},
            ]},
        }],
    }

class LoopProofV4Tests(unittest.TestCase):
    def test_countdown_generates_all_loop_vcs(self):
        rendered = generate_typed_smtlib(document())
        for token in ("loop-init", "loop-preserve", "loop-decreases-nonnegative", "loop-decreases-strict", "loop-exit"):
            self.assertIn(token, rendered)
    def test_unannotated_cycle_is_rejected(self):
        value = document(); value["functions"][0]["body"]["blocks"][1].pop("loop")
        with self.assertRaisesRegex(TypedVerifiedCoreError, "cyclic CFG"): validate_typed_document(value)
    def test_non_boolean_invariant_is_rejected(self):
        value = document(); value["functions"][0]["body"]["blocks"][1]["loop"]["invariants"] = [expr(20, "local", 1, local_id=0)]
        with self.assertRaisesRegex(TypedVerifiedCoreError, "invariant must have type bool"): validate_typed_document(value)

if __name__ == "__main__": unittest.main()
