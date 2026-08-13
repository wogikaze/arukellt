#!/usr/bin/env python3
"""Exercise Phase 3 modular direct-call verification with exact interface binding."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_typed_v1 import generate_typed_smtlib
from proof.verified_core_interface import bind_call_interfaces


def contract(kind: str, base: int) -> dict:
    left = {"id": base + 1, "kind": "result" if kind == "ensures" else "local", "type_id": 1}
    if kind == "requires":
        left["local_id"] = 0
    value = {
        "kind": kind,
        "expression": {
            "id": base,
            "kind": "ge",
            "type_id": 2,
            "operands": [left, {"id": base + 2, "kind": "constant", "type_id": 1, "value": 0}],
        },
    }
    if kind == "ensures":
        value["result_name"] = "result"
    return value


def function(function_id: int, name: str, callee_id: int | None) -> dict:
    locals_ = [{"id": 0, "name": "x", "type_id": 1, "storage": "parameter"}]
    instructions = []
    returned = {"kind": "local", "type_id": 1, "local_id": 0}
    if callee_id is not None:
        locals_.append({"id": 1, "name": "y", "type_id": 1, "storage": "temporary"})
        instructions.append({
            "id": 0,
            "op": "call",
            "callee_id": callee_id,
            "dest_local_id": 1,
            "type_id": 1,
            "arguments": [{"kind": "local", "type_id": 1, "local_id": 0}],
        })
        returned = {"kind": "local", "type_id": 1, "local_id": 1}
    return {
        "id": function_id,
        "name": name,
        "signature": {"parameters": [{"name": "x", "type_id": 1}], "return_type_id": 1},
        "abi": {
            "calling_convention": "arukellt",
            "parameters": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}],
            "results": [{"type_id": 1, "passing": "value", "wasm": ["i32"]}],
        },
        "locals": locals_,
        "contracts": [contract("requires", 0), contract("ensures", 3)],
        "body": {
            "entry_block": 0,
            "blocks": [{
                "id": 0,
                "parameters": [],
                "instructions": instructions,
                "terminator": {"kind": "return", "value": returned},
            }],
        },
    }


def main() -> int:
    document = {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": "phase3-check",
        "module": "phase3",
        "target_profile": {
            "integer_model": "mathematical",
            "overflow": "checked",
            "floating_point": "unsupported",
            "pointer_width": 32,
        },
        "types": [
            {"id": 0, "kind": "unit", "name": "()", "representation": {"wasm": [], "nullable": False, "size_bytes": 0, "align_bytes": 1}},
            {"id": 1, "kind": "integer", "name": "i32", "bits": 32, "signed": True, "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
            {"id": 2, "kind": "bool", "name": "bool", "representation": {"wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4}},
        ],
        "functions": [
            function(0, "nonnegative_identity", None),
            function(1, "forward_nonnegative", 0),
        ],
    }
    bind_call_interfaces(document)
    rendered = generate_typed_smtlib(document)
    if "callee-requires" not in rendered:
        raise ValueError("callee requires did not become a caller obligation")
    if "callee-interface-sha256" not in rendered:
        raise ValueError("callee interface digest is missing from solver input")
    if "f1_forward_nonnegative_b0_i0_call0" not in rendered:
        raise ValueError("callee result was not represented as a modular call value")
    print(f"proof-phase3-modular: PASS: obligations={rendered.count('(check-sat)')}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, TypeError, KeyError) as exc:
        print(f"proof-phase3-modular: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
