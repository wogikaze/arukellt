"""Phase 5 aggregate instruction and document semantic checks."""
from __future__ import annotations
import copy
from typing import Any
from proof import verified_core_typed_impl as legacy
from proof.aggregate_types import is_aggregate_type, validate_aggregate_types
from proof.aggregate_semantics import AGGREGATE_INSTRUCTION_OPS, _bool_type, _expression_type, _member_types, _types, _value_type

def _validate_instruction(instruction, *, path, types, local_types):
    op = str(instruction["op"])
    if op not in AGGREGATE_INSTRUCTION_OPS:
        if op == "binary" and instruction.get("operator") in {"eq", "ne"}:
            args = instruction.get("arguments", [])
            if len(args) == 2 and int(args[0]["type_id"]) in types and is_aggregate_type(types[int(args[0]["type_id"])]):
                if int(args[1]["type_id"]) != int(args[0]["type_id"]) or int(instruction["type_id"]) != _bool_type(types): raise ValueError(f"{path}: aggregate equality type mismatch")
                return True
        return False
    dest_type = int(instruction["type_id"])
    if local_types.get(int(instruction["dest_local_id"])) != dest_type: raise ValueError(f"{path}.dest_local_id: destination type mismatch")
    arg_types = [_value_type(value, local_types, types, f"{path}.arguments[{i}]") for i, value in enumerate(instruction.get("arguments", []))]
    if op == "construct":
        entry = types[dest_type]
        if not is_aggregate_type(entry): raise ValueError(f"{path}.type_id: construct destination must be aggregate")
        variant_index = instruction.get("variant_index")
        if entry["kind"] == "enum" and type(variant_index) is not int: raise ValueError(f"{path}.variant_index: enum constructor requires index")
        if entry["kind"] != "enum" and variant_index is not None: raise ValueError(f"{path}.variant_index: only enum constructor uses variant index")
        if arg_types != _member_types(entry, variant_index): raise ValueError(f"{path}.arguments: constructor type mismatch")
        return True
    if len(arg_types) != 1: raise ValueError(f"{path}.arguments: aggregate operation requires one source")
    source = types[arg_types[0]]
    if op == "project":
        members = _member_types(source); index = instruction.get("index")
        if source["kind"] not in {"tuple", "struct"} or type(index) is not int or index < 0 or index >= len(members) or dest_type != members[index]: raise ValueError(f"{path}.index: projection mismatch")
    elif op == "is_variant":
        if source["kind"] != "enum" or dest_type != _bool_type(types): raise ValueError(f"{path}: is_variant mismatch")
        _member_types(source, instruction.get("variant_index"))
    else:
        if source["kind"] != "enum": raise ValueError(f"{path}: variant_payload requires enum")
        members = _member_types(source, instruction.get("variant_index")); payload_index = instruction.get("payload_index")
        if type(payload_index) is not int or payload_index < 0 or payload_index >= len(members) or dest_type != members[payload_index]: raise ValueError(f"{path}.payload_index: variant payload mismatch")
    return True

def validate_aggregate_semantics(document: dict[str, Any]) -> None:
    validate_aggregate_types(document); types = _types(document); kinds = {tid: str(entry["kind"]) for tid, entry in types.items()}; functions = {int(function["id"]): function for function in document["functions"]}
    for function_index, function in enumerate(document["functions"]):
        path = f"$.functions[{function_index}]"; local_types = {int(local["id"]): int(local["type_id"]) for local in function["locals"]}; return_type = int(function["signature"]["return_type_id"]); seen_ids: set[int] = set()
        for contract_index, contract in enumerate(function["contracts"]):
            expression_type = _expression_type(contract["expression"], path=f"{path}.contracts[{contract_index}].expression", types=types, local_types=local_types, return_type=return_type, allow_result=contract["kind"] == "ensures", seen_ids=seen_ids)
            if contract["kind"] == "decreases":
                if kinds[expression_type] != "integer": raise ValueError(f"{path}.contracts[{contract_index}]: decreases must be integer")
            elif kinds[expression_type] != "bool": raise ValueError(f"{path}.contracts[{contract_index}]: contract must have type bool")
        for block_index, block in enumerate(function["body"]["blocks"]):
            for instruction_index, instruction in enumerate(block["instructions"]):
                handled = _validate_instruction(instruction, path=f"{path}.body.blocks[{block_index}].instructions[{instruction_index}]", types=types, local_types=local_types)
                if not handled: legacy._check_instruction(instruction, path=f"{path}.body.blocks[{block_index}].instructions[{instruction_index}]", local_types=local_types, kinds=kinds, functions=functions)
            term = block["terminator"]
            if term["kind"] == "return" and "value" in term and _value_type(term["value"], local_types, types, f"{path}.body.blocks[{block_index}].terminator.value") != return_type: raise ValueError(f"{path}.body.blocks[{block_index}].terminator.value: return type mismatch")
    legacy._check_call_graph(functions)

def _contains_aggregate_expr(expression, aggregate_ids):
    if int(expression.get("type_id", -1)) in aggregate_ids or expression.get("kind") in {"construct", "project", "is_variant", "variant_payload"}: return True
    return any(_contains_aggregate_expr(op, aggregate_ids) for op in expression.get("operands", []))

def compatibility_view(document: dict[str, Any]) -> dict[str, Any]:
    value = copy.deepcopy(document); types = _types(value); aggregate_ids = {tid for tid, entry in types.items() if is_aggregate_type(entry)}; bool_id = _bool_type(types)
    for function in value["functions"]:
        for contract in function["contracts"]:
            expression = contract["expression"]
            if _contains_aggregate_expr(expression, aggregate_ids): contract["expression"] = {"id": int(expression["id"]), "kind": "constant", "type_id": bool_id, "value": True}
        for block in function["body"]["blocks"]:
            kept = []
            for instruction in block["instructions"]:
                op = instruction["op"]
                if op in AGGREGATE_INSTRUCTION_OPS: continue
                if op == "binary" and instruction.get("operator") in {"eq", "ne"}:
                    args = instruction.get("arguments", [])
                    if args and int(args[0]["type_id"]) in aggregate_ids: continue
                kept.append(instruction)
            block["instructions"] = kept
    return value

__all__ = ["compatibility_view", "validate_aggregate_semantics"]
