"""Proof Phase 7 read-only reference and heap semantics."""
from __future__ import annotations

import copy
from typing import Any

from proof.aggregate_types import is_aggregate_type
from proof.aggregate_semantics import _bool_type, _member_types
from proof.machine_integer_semantics import integer_bounds

MODEL = "arukellt-readonly-heap-v1"
MEMORY_EXPRESSION_KINDS = {"is_null", "ref_eq", "load_field", "array_len", "array_get"}
MEMORY_INSTRUCTION_OPS = {"is_null", "ref_eq", "load_field", "array_len", "array_get"}
MUTATING_MEMORY_OPS = {"store_field", "array_set", "store", "alloc", "free", "gc_alloc"}


def _fail(path: str, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def _types(document: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {int(entry["id"]): entry for entry in document.get("types", [])}


def _reference_descriptors(document: dict[str, Any]) -> dict[int, dict[str, Any]]:
    memory = document.get("proof_memory")
    if not isinstance(memory, dict):
        _fail("$.proof_memory", "expected object")
    if set(memory) != {"model", "references"}:
        _fail("$.proof_memory", "expected exactly model/references")
    if memory.get("model") != MODEL:
        _fail("$.proof_memory.model", f"expected {MODEL!r}")
    references = memory.get("references")
    if not isinstance(references, list):
        _fail("$.proof_memory.references", "expected array")
    result: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(references):
        path = f"$.proof_memory.references[{index}]"
        if not isinstance(raw, dict):
            _fail(path, "expected object")
        type_id = int(raw.get("type_id", -1))
        if type_id in result:
            _fail(f"{path}.type_id", "duplicate reference descriptor")
        result[type_id] = raw
    return result


def _validate_reference_descriptors(document: dict[str, Any]) -> dict[int, dict[str, Any]]:
    types = _types(document)
    descriptors = _reference_descriptors(document)
    reference_ids = {type_id for type_id, entry in types.items() if entry.get("kind") == "reference"}
    if reference_ids != set(descriptors):
        missing = sorted(reference_ids - set(descriptors))
        extra = sorted(set(descriptors) - reference_ids)
        _fail("$.proof_memory.references", f"reference TypeId set mismatch missing={missing} extra={extra}")
    for type_id in sorted(reference_ids):
        entry = types[type_id]
        descriptor = descriptors[type_id]
        path = f"$.proof_memory.references[type_id={type_id}]"
        representation = entry.get("representation")
        if not isinstance(representation, dict) or type(representation.get("nullable")) is not bool:
            _fail(f"$.types[id={type_id}].representation.nullable", "explicit boolean nullability required")
        wasm = representation.get("wasm")
        if not isinstance(wasm, list) or not wasm:
            _fail(f"$.types[id={type_id}].representation.wasm", "explicit reference representation required")
        pointee = int(entry.get("pointee_type_id", -1))
        if pointee not in types:
            _fail(f"$.types[id={type_id}].pointee_type_id", "unknown pointee TypeId")
        kind = str(descriptor.get("kind", ""))
        if kind == "object":
            if set(descriptor) != {"type_id", "kind", "pointee_type_id"}:
                _fail(path, "object descriptor requires type_id/kind/pointee_type_id")
            if int(descriptor["pointee_type_id"]) != pointee:
                _fail(f"{path}.pointee_type_id", "must match reference TypeId metadata")
            if types[pointee].get("kind") != "struct":
                _fail(f"{path}.pointee_type_id", "object pointee must be struct")
        elif kind in {"array", "slice"}:
            if set(descriptor) != {"type_id", "kind", "element_type_id", "length_type_id"}:
                _fail(path, "array/slice descriptor requires type_id/kind/element_type_id/length_type_id")
            element_type = int(descriptor["element_type_id"])
            length_type = int(descriptor["length_type_id"])
            if element_type not in types:
                _fail(f"{path}.element_type_id", "unknown TypeId")
            if length_type not in types or types[length_type].get("kind") != "integer":
                _fail(f"{path}.length_type_id", "length TypeId must be integer")
        else:
            _fail(f"{path}.kind", f"unsupported read-only reference kind {kind!r}")
    return descriptors


def _local_types(function: dict[str, Any]) -> dict[int, int]:
    return {int(local["id"]): int(local["type_id"]) for local in function.get("locals", [])}


def _value_type(value: dict[str, Any], local_types: dict[int, int], types: dict[int, dict[str, Any]], path: str) -> int:
    if not isinstance(value, dict):
        _fail(path, "expected typed value")
    type_id = int(value.get("type_id", -1))
    if type_id not in types:
        _fail(f"{path}.type_id", "unknown TypeId")
    kind = value.get("kind")
    if kind == "local":
        if local_types.get(int(value.get("local_id", -1))) != type_id:
            _fail(f"{path}.local_id", "local TypeId mismatch")
    elif kind == "constant":
        type_kind = types[type_id].get("kind")
        if type_kind == "reference" or is_aggregate_type(types[type_id]):
            _fail(path, "reference/aggregate constants are not admitted; use typed operations")
        raw = value.get("value")
        if type_kind == "integer":
            if type(raw) is not int:
                _fail(f"{path}.value", "integer required")
            low, high = integer_bounds(types[type_id])
            if raw < low or raw > high:
                _fail(f"{path}.value", "integer constant outside machine range")
        elif type_kind == "bool" and type(raw) is not bool:
            _fail(f"{path}.value", "boolean required")
    else:
        _fail(f"{path}.kind", "expected local or constant")
    return type_id


def _contains_memory_expression(expression: dict[str, Any], types: dict[int, dict[str, Any]]) -> bool:
    if not isinstance(expression, dict):
        return False
    if expression.get("kind") in MEMORY_EXPRESSION_KINDS:
        return True
    type_id = expression.get("type_id")
    if type(type_id) is int and type_id in types and types[type_id].get("kind") == "reference":
        return True
    return any(_contains_memory_expression(operand, types) for operand in expression.get("operands", []))


def _expression_type(
    expression: dict[str, Any],
    *,
    path: str,
    types: dict[int, dict[str, Any]],
    descriptors: dict[int, dict[str, Any]],
    local_types: dict[int, int],
    return_type: int,
    allow_result: bool,
    seen_ids: set[int],
) -> int:
    if not isinstance(expression, dict):
        _fail(path, "expected expression")
    expression_id = int(expression.get("id", -1))
    if expression_id < 0 or expression_id in seen_ids:
        _fail(f"{path}.id", "invalid or duplicate expression id")
    seen_ids.add(expression_id)
    kind = str(expression.get("kind", ""))
    type_id = int(expression.get("type_id", -1))
    if type_id not in types:
        _fail(f"{path}.type_id", "unknown TypeId")
    operands = expression.get("operands", [])
    if not isinstance(operands, list):
        _fail(f"{path}.operands", "expected array")

    if kind == "local":
        if operands or local_types.get(int(expression.get("local_id", -1))) != type_id:
            _fail(path, "local TypeId mismatch")
        return type_id
    if kind == "result":
        if not allow_result or operands or return_type == 0 or type_id != return_type:
            _fail(path, "invalid result expression")
        return type_id
    if kind == "constant":
        if operands:
            _fail(f"{path}.operands", "constant must be leaf")
        _value_type({"kind": "constant", "type_id": type_id, "value": expression.get("value")}, local_types, types, path)
        return type_id

    operand_types = [
        _expression_type(
            operand,
            path=f"{path}.operands[{index}]",
            types=types,
            descriptors=descriptors,
            local_types=local_types,
            return_type=return_type,
            allow_result=allow_result,
            seen_ids=seen_ids,
        )
        for index, operand in enumerate(operands)
    ]

    if kind == "is_null":
        if len(operand_types) != 1 or types[operand_types[0]].get("kind") != "reference" or type_id != _bool_type(types):
            _fail(path, "is_null type mismatch")
        return type_id
    if kind == "ref_eq":
        if len(operand_types) != 2 or operand_types[0] != operand_types[1] or types[operand_types[0]].get("kind") != "reference" or type_id != _bool_type(types):
            _fail(path, "ref_eq requires same reference TypeId and bool result")
        return type_id
    if kind == "load_field":
        if len(operand_types) != 1:
            _fail(path, "load_field requires one reference")
        reference_type = operand_types[0]
        descriptor = descriptors.get(reference_type)
        if not descriptor or descriptor.get("kind") != "object":
            _fail(path, "load_field requires object reference")
        fields = types[int(descriptor["pointee_type_id"])].get("fields", [])
        index = expression.get("field_index")
        if type(index) is not int or index < 0 or index >= len(fields) or type_id != int(fields[index]["type_id"]):
            _fail(f"{path}.field_index", "field projection type mismatch")
        return type_id
    if kind == "array_len":
        if len(operand_types) != 1:
            _fail(path, "array_len requires one reference")
        descriptor = descriptors.get(operand_types[0])
        if not descriptor or descriptor.get("kind") not in {"array", "slice"} or type_id != int(descriptor["length_type_id"]):
            _fail(path, "array_len type mismatch")
        return type_id
    if kind == "array_get":
        if len(operand_types) != 2:
            _fail(path, "array_get requires reference and index")
        descriptor = descriptors.get(operand_types[0])
        if not descriptor or descriptor.get("kind") not in {"array", "slice"}:
            _fail(path, "array_get requires array/slice reference")
        if operand_types[1] != int(descriptor["length_type_id"]) or type_id != int(descriptor["element_type_id"]):
            _fail(path, "array_get TypeId mismatch")
        return type_id

    if kind == "construct":
        entry = types[type_id]
        if not is_aggregate_type(entry):
            _fail(path, "construct destination must be aggregate")
        variant = expression.get("variant_index")
        expected = _member_types(entry, variant) if entry["kind"] == "enum" else _member_types(entry)
        if entry["kind"] != "enum" and variant is not None:
            _fail(f"{path}.variant_index", "only enum constructor uses variant index")
        if operand_types != expected:
            _fail(f"{path}.operands", "constructor type mismatch")
        return type_id
    if kind == "project":
        if len(operand_types) != 1 or types[operand_types[0]].get("kind") not in {"tuple", "struct"}:
            _fail(path, "projection requires tuple/struct")
        members = _member_types(types[operand_types[0]])
        index = expression.get("index")
        if type(index) is not int or index < 0 or index >= len(members) or type_id != members[index]:
            _fail(f"{path}.index", "projection type mismatch")
        return type_id
    if kind == "is_variant":
        if len(operand_types) != 1 or types[operand_types[0]].get("kind") != "enum" or type_id != _bool_type(types):
            _fail(path, "variant test type mismatch")
        _member_types(types[operand_types[0]], expression.get("variant_index"))
        return type_id
    if kind == "variant_payload":
        if len(operand_types) != 1 or types[operand_types[0]].get("kind") != "enum":
            _fail(path, "variant payload requires enum")
        members = _member_types(types[operand_types[0]], expression.get("variant_index"))
        payload = expression.get("payload_index")
        if type(payload) is not int or payload < 0 or payload >= len(members) or type_id != members[payload]:
            _fail(f"{path}.payload_index", "variant payload type mismatch")
        return type_id

    if kind in {"neg", "not"}:
        if len(operand_types) != 1:
            _fail(f"{path}.operands", "unary arity")
        source = operand_types[0]
        if kind == "neg":
            if types[source].get("kind") != "integer" or type_id != source:
                _fail(path, "neg type mismatch")
        elif types[source].get("kind") != "bool" or types[type_id].get("kind") != "bool":
            _fail(path, "not type mismatch")
        return type_id

    binary = {"add", "sub", "mul", "div", "mod", "eq", "ne", "lt", "le", "gt", "ge", "and", "or", "implies"}
    if kind not in binary or len(operand_types) != 2:
        _fail(f"{path}.kind", f"unsupported expression {kind!r}")
    left, right = operand_types
    if left != right:
        _fail(f"{path}.operands", "TypeId mismatch")
    source_kind = types[left].get("kind")
    bool_id = _bool_type(types)
    if kind in {"add", "sub", "mul", "div", "mod"}:
        if source_kind != "integer" or type_id != left:
            _fail(path, "arithmetic type mismatch")
    elif kind in {"lt", "le", "gt", "ge"}:
        if source_kind != "integer" or type_id != bool_id:
            _fail(path, "comparison type mismatch")
    elif kind in {"eq", "ne"}:
        if source_kind not in {"integer", "bool"} and not is_aggregate_type(types[left]):
            _fail(path, "scalar/aggregate equality type mismatch; references require ref_eq")
        if type_id != bool_id:
            _fail(path, "equality result must be bool")
    elif source_kind != "bool" or type_id != bool_id:
        _fail(path, "logical type mismatch")
    return type_id


def _validate_memory_instruction(
    instruction: dict[str, Any],
    *,
    path: str,
    local_types: dict[int, int],
    types: dict[int, dict[str, Any]],
    descriptors: dict[int, dict[str, Any]],
) -> bool:
    op = str(instruction.get("op", ""))
    if op in MUTATING_MEMORY_OPS:
        _fail(f"{path}.op", "mutation/allocation is outside read-only Phase 7")
    if op not in MEMORY_INSTRUCTION_OPS:
        return False
    dest_type = int(instruction.get("type_id", -1))
    if local_types.get(int(instruction.get("dest_local_id", -1))) != dest_type:
        _fail(f"{path}.dest_local_id", "destination TypeId mismatch")
    args = instruction.get("arguments", [])
    if not isinstance(args, list):
        _fail(f"{path}.arguments", "expected array")
    arg_types = [_value_type(value, local_types, types, f"{path}.arguments[{index}]") for index, value in enumerate(args)]
    if op == "is_null":
        if len(arg_types) != 1 or types[arg_types[0]].get("kind") != "reference" or dest_type != _bool_type(types):
            _fail(path, "is_null type mismatch")
    elif op == "ref_eq":
        if len(arg_types) != 2 or arg_types[0] != arg_types[1] or types[arg_types[0]].get("kind") != "reference" or dest_type != _bool_type(types):
            _fail(path, "ref_eq type mismatch")
    elif op == "load_field":
        if len(arg_types) != 1:
            _fail(path, "load_field requires one reference")
        descriptor = descriptors.get(arg_types[0])
        if not descriptor or descriptor.get("kind") != "object":
            _fail(path, "load_field requires object reference")
        fields = types[int(descriptor["pointee_type_id"])].get("fields", [])
        index = instruction.get("field_index")
        if type(index) is not int or index < 0 or index >= len(fields) or dest_type != int(fields[index]["type_id"]):
            _fail(f"{path}.field_index", "field projection type mismatch")
    elif op == "array_len":
        if len(arg_types) != 1:
            _fail(path, "array_len requires one reference")
        descriptor = descriptors.get(arg_types[0])
        if not descriptor or descriptor.get("kind") not in {"array", "slice"} or dest_type != int(descriptor["length_type_id"]):
            _fail(path, "array_len type mismatch")
    elif op == "array_get":
        if len(arg_types) != 2:
            _fail(path, "array_get requires reference and index")
        descriptor = descriptors.get(arg_types[0])
        if not descriptor or descriptor.get("kind") not in {"array", "slice"}:
            _fail(path, "array_get requires array/slice reference")
        if arg_types[1] != int(descriptor["length_type_id"]) or dest_type != int(descriptor["element_type_id"]):
            _fail(path, "array_get TypeId mismatch")
    return True


def function_uses_memory(function: dict[str, Any], types: dict[int, dict[str, Any]]) -> bool:
    reference_ids = {type_id for type_id, entry in types.items() if entry.get("kind") == "reference"}
    if int(function["signature"]["return_type_id"]) in reference_ids:
        return True
    if any(int(parameter["type_id"]) in reference_ids for parameter in function["signature"]["parameters"]):
        return True
    if any(int(local["type_id"]) in reference_ids for local in function["locals"]):
        return True
    if any(_contains_memory_expression(contract["expression"], types) for contract in function["contracts"]):
        return True
    return any(
        instruction.get("op") in MEMORY_INSTRUCTION_OPS
        for block in function["body"]["blocks"]
        for instruction in block["instructions"]
    )


def validate_readonly_memory_semantics(document: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(document, dict):
        _fail("$", "expected object")
    types = _types(document)
    descriptors = _validate_reference_descriptors(document)
    for function_index, function in enumerate(document.get("functions", [])):
        path = f"$.functions[{function_index}]"
        local_types = _local_types(function)
        return_type = int(function["signature"]["return_type_id"])
        seen_ids: set[int] = set()
        for contract_index, contract in enumerate(function.get("contracts", [])):
            if contract.get("kind") == "decreases" and _contains_memory_expression(contract["expression"], types):
                _fail(f"{path}.contracts[{contract_index}]", "memory-dependent decreases is outside Phase 7")
            expression_type = _expression_type(
                contract["expression"],
                path=f"{path}.contracts[{contract_index}].expression",
                types=types,
                descriptors=descriptors,
                local_types=local_types,
                return_type=return_type,
                allow_result=contract.get("kind") == "ensures",
                seen_ids=seen_ids,
            )
            if contract.get("kind") == "decreases":
                if types[expression_type].get("kind") != "integer":
                    _fail(f"{path}.contracts[{contract_index}]", "decreases must be integer")
            elif types[expression_type].get("kind") != "bool":
                _fail(f"{path}.contracts[{contract_index}]", "contract must be bool")

        uses_memory = function_uses_memory(function, types)
        blocks = function.get("body", {}).get("blocks", [])
        if uses_memory:
            if len(blocks) != 1 or int(function["body"]["entry_block"]) != int(blocks[0]["id"]):
                _fail(f"{path}.body", "Phase 7 memory functions require one straight-line block")
            if blocks[0].get("terminator", {}).get("kind") != "return":
                _fail(f"{path}.body", "Phase 7 memory function requires direct return")
            if "loop" in blocks[0]:
                _fail(f"{path}.body", "memory + loop composition is outside Phase 7")
        for block_index, block in enumerate(blocks):
            for instruction_index, instruction in enumerate(block.get("instructions", [])):
                if uses_memory and instruction.get("op") == "call":
                    _fail(f"{path}.body.blocks[{block_index}].instructions[{instruction_index}]", "memory + call composition is outside Phase 7")
                _validate_memory_instruction(
                    instruction,
                    path=f"{path}.body.blocks[{block_index}].instructions[{instruction_index}]",
                    local_types=local_types,
                    types=types,
                    descriptors=descriptors,
                )
    return document


def compatibility_view(document: dict[str, Any]) -> dict[str, Any]:
    value = copy.deepcopy(document)
    value.pop("proof_memory", None)
    types = _types(value)
    bool_id = _bool_type(types)
    original_types = _types(document)
    for function in value.get("functions", []):
        for contract in function.get("contracts", []):
            expression = contract["expression"]
            if _contains_memory_expression(expression, original_types):
                contract["expression"] = {"id": int(expression["id"]), "kind": "constant", "type_id": bool_id, "value": True}
        for block in function.get("body", {}).get("blocks", []):
            block["instructions"] = [instruction for instruction in block.get("instructions", []) if instruction.get("op") not in MEMORY_INSTRUCTION_OPS]
    return value


__all__ = [
    "MEMORY_EXPRESSION_KINDS",
    "MEMORY_INSTRUCTION_OPS",
    "MODEL",
    "compatibility_view",
    "function_uses_memory",
    "validate_readonly_memory_semantics",
]
