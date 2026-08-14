"""Validation-only legacy view for independently checked Phase 5 aggregates."""
from __future__ import annotations
import copy
from typing import Any
from proof.aggregate_semantics import AGGREGATE_EXPRESSION_KINDS, AGGREGATE_INSTRUCTION_OPS
from proof.aggregate_types import is_aggregate_type


def _contains_aggregate_expr(expression: dict[str, Any], aggregate_ids: set[int]) -> bool:
    if int(expression.get("type_id", -1)) in aggregate_ids or expression.get("kind") in AGGREGATE_EXPRESSION_KINDS:
        return True
    return any(_contains_aggregate_expr(operand, aggregate_ids) for operand in expression.get("operands", []))


def compatibility_view(document: dict[str, Any]) -> dict[str, Any]:
    value = copy.deepcopy(document)
    aggregate_ids = {int(entry["id"]) for entry in value["types"] if is_aggregate_type(entry)}
    bool_ids = [int(entry["id"]) for entry in value["types"] if entry["kind"] == "bool"]
    if len(bool_ids) != 1:
        raise ValueError("$.types: exactly one bool type is required")
    bool_id = bool_ids[0]
    for entry in value["types"]:
        if int(entry["id"]) not in aggregate_ids:
            continue
        type_id = int(entry["id"])
        name = str(entry.get("name", f"aggregate-{type_id}"))
        entry.clear()
        entry.update({
            "id": type_id,
            "kind": "unit",
            "name": name,
            "representation": {"wasm": [], "nullable": False, "size_bytes": 0, "align_bytes": 1},
        })
    for function in value["functions"]:
        for contract in function["contracts"]:
            expression = contract["expression"]
            if _contains_aggregate_expr(expression, aggregate_ids):
                contract["expression"] = {"id": int(expression["id"]), "kind": "constant", "type_id": bool_id, "value": True}
        for block in function["body"]["blocks"]:
            block.pop("loop", None)
            kept = []
            for instruction in block["instructions"]:
                op = instruction["op"]
                if op in AGGREGATE_INSTRUCTION_OPS:
                    continue
                if op == "binary" and instruction.get("operator") in {"eq", "ne"}:
                    arguments = instruction.get("arguments", [])
                    if arguments and int(arguments[0]["type_id"]) in aggregate_ids:
                        continue
                if op == "call":
                    instruction.pop("callee_interface_sha256", None)
                kept.append(instruction)
            block["instructions"] = kept
    return value


__all__ = ["compatibility_view"]
