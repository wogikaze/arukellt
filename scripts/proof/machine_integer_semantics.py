"""Proof Phase 6 machine-integer semantic admission.

The proof model is versioned as ``arukellt-machine-int-v1``. Integer values are
represented with SMT ``Int`` terms plus explicit signed range constraints. Type
meaning is derived only from TypeId metadata (bits/signed/representation), never
from display names.
"""
from __future__ import annotations

import copy
from typing import Any

PROFILE = "arukellt-machine-int-v1"
SHIFT_OPERATORS = {"shl", "shr_s"}
CONVERSIONS = {"signed-widen", "checked-narrow"}
_ARITHMETIC_EXPRESSIONS = {"neg", "add", "sub", "mul", "div", "mod"}


def _fail(path: str, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def _types(document: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {int(entry["id"]): entry for entry in document.get("types", [])}


def integer_bounds(entry: dict[str, Any]) -> tuple[int, int]:
    bits = int(entry["bits"])
    return -(1 << (bits - 1)), (1 << (bits - 1)) - 1


def integer_type(document: dict[str, Any], type_id: int, path: str) -> dict[str, Any]:
    types = _types(document)
    entry = types.get(type_id)
    if not isinstance(entry, dict) or entry.get("kind") != "integer":
        _fail(path, "expected integer TypeId")
    return entry


def _bool_type_id(types: dict[int, dict[str, Any]]) -> int:
    matches = [type_id for type_id, entry in types.items() if entry.get("kind") == "bool"]
    if len(matches) != 1:
        _fail("$.types", "machine profile requires exactly one bool TypeId")
    return matches[0]


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
        _validate_constant(value.get("value"), type_id, types, f"{path}.value")
    else:
        _fail(f"{path}.kind", "expected local or constant")
    return type_id


def _validate_integer_metadata(entry: dict[str, Any], path: str) -> None:
    bits = entry.get("bits")
    signed = entry.get("signed")
    if type(bits) is not int or bits not in {32, 64}:
        _fail(f"{path}.bits", "expected 32 or 64")
    if signed is not True:
        _fail(f"{path}.signed", "Phase 6 supports signed machine integers only")
    representation = entry.get("representation")
    if not isinstance(representation, dict):
        _fail(f"{path}.representation", "expected object")
    expected = [f"i{bits}"]
    if representation.get("wasm") != expected:
        _fail(f"{path}.representation.wasm", f"expected {expected!r}")
    if representation.get("nullable") is not False:
        _fail(f"{path}.representation.nullable", "machine integer must be non-null")


def _validate_constant(value: Any, type_id: int, types: dict[int, dict[str, Any]], path: str) -> None:
    entry = types[type_id]
    kind = entry.get("kind")
    if kind == "integer":
        if type(value) is not int:
            _fail(path, "integer constant requires integer")
        low, high = integer_bounds(entry)
        if value < low or value > high:
            _fail(path, f"constant {value} is outside signed {entry['bits']}-bit range")
    elif kind == "bool":
        if type(value) is not bool:
            _fail(path, "bool constant requires boolean")


def _walk_expression(expression: dict[str, Any], types: dict[int, dict[str, Any]], path: str, *, decreases: bool = False) -> None:
    if not isinstance(expression, dict):
        _fail(path, "expected expression")
    type_id = int(expression.get("type_id", -1))
    if type_id not in types:
        _fail(f"{path}.type_id", "unknown TypeId")
    kind = str(expression.get("kind", ""))
    if kind == "constant":
        _validate_constant(expression.get("value"), type_id, types, f"{path}.value")
    if decreases and kind in _ARITHMETIC_EXPRESSIONS:
        _fail(path, "Phase 6 decreases must be a non-arithmetic integer expression")
    for index, operand in enumerate(expression.get("operands", [])):
        _walk_expression(operand, types, f"{path}.operands[{index}]", decreases=decreases)


def _validate_instruction(
    instruction: dict[str, Any],
    *,
    path: str,
    local_types: dict[int, int],
    types: dict[int, dict[str, Any]],
) -> None:
    op = str(instruction.get("op", ""))
    dest = int(instruction.get("dest_local_id", -1))
    dest_type = int(instruction.get("type_id", -1))
    if local_types.get(dest) != dest_type or dest_type not in types:
        _fail(f"{path}.dest_local_id", "destination TypeId mismatch")
    if op == "const":
        _validate_constant(instruction.get("value"), dest_type, types, f"{path}.value")
        return
    arguments = instruction.get("arguments", [])
    if not isinstance(arguments, list):
        _fail(f"{path}.arguments", "expected array")
    arg_types = [_value_type(value, local_types, types, f"{path}.arguments[{index}]") for index, value in enumerate(arguments)]
    if op == "binary" and str(instruction.get("operator")) in SHIFT_OPERATORS:
        if len(arg_types) != 2 or arg_types[0] != arg_types[1] or dest_type != arg_types[0]:
            _fail(path, "machine shift requires matching integer TypeIds")
        if types[dest_type].get("kind") != "integer":
            _fail(path, "machine shift requires integer TypeId")
        shift = arguments[1]
        if shift.get("kind") != "constant" or type(shift.get("value")) is not int:
            _fail(f"{path}.arguments[1]", "Phase 6 shifts require a constant count")
        return
    if op == "convert":
        if len(arg_types) != 1:
            _fail(f"{path}.arguments", "conversion requires one source")
        source = types[arg_types[0]]
        target = types[dest_type]
        if source.get("kind") != "integer" or target.get("kind") != "integer":
            _fail(path, "conversion requires integer source/destination")
        conversion = str(instruction.get("conversion", ""))
        if conversion not in CONVERSIONS:
            _fail(f"{path}.conversion", f"unsupported conversion {conversion!r}")
        source_bits = int(source["bits"])
        target_bits = int(target["bits"])
        if conversion == "signed-widen" and not (source_bits == 32 and target_bits == 64):
            _fail(path, "signed-widen requires i32 -> i64")
        if conversion == "checked-narrow" and not (source_bits == 64 and target_bits == 32):
            _fail(path, "checked-narrow requires i64 -> i32")


def validate_machine_integer_semantics(document: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(document, dict):
        _fail("$", "expected object")
    profile = document.get("target_profile")
    if not isinstance(profile, dict):
        _fail("$.target_profile", "expected object")
    if profile.get("integer_model") != "machine":
        _fail("$.target_profile.integer_model", "Phase 6 requires 'machine'")
    if profile.get("overflow") != "checked":
        _fail("$.target_profile.overflow", "Phase 6 requires checked overflow")
    if profile.get("floating_point") != "unsupported":
        _fail("$.target_profile.floating_point", "Phase 6 requires floating point unsupported")

    types = _types(document)
    if not types:
        _fail("$.types", "expected non-empty type table")
    _bool_type_id(types)
    for index, entry in enumerate(document["types"]):
        if entry.get("kind") == "integer":
            _validate_integer_metadata(entry, f"$.types[{index}]")

    for function_index, function in enumerate(document.get("functions", [])):
        path = f"$.functions[{function_index}]"
        local_types = _local_types(function)
        for contract_index, contract in enumerate(function.get("contracts", [])):
            _walk_expression(
                contract["expression"],
                types,
                f"{path}.contracts[{contract_index}].expression",
                decreases=contract.get("kind") == "decreases",
            )
        for block_index, block in enumerate(function.get("body", {}).get("blocks", [])):
            if "loop" in block:
                for invariant_index, expression in enumerate(block["loop"].get("invariants", [])):
                    _walk_expression(expression, types, f"{path}.body.blocks[{block_index}].loop.invariants[{invariant_index}]")
                _walk_expression(block["loop"]["decreases"], types, f"{path}.body.blocks[{block_index}].loop.decreases", decreases=True)
            for instruction_index, instruction in enumerate(block.get("instructions", [])):
                _validate_instruction(
                    instruction,
                    path=f"{path}.body.blocks[{block_index}].instructions[{instruction_index}]",
                    local_types=local_types,
                    types=types,
                )
    return document


def compatibility_view(document: dict[str, Any]) -> dict[str, Any]:
    """Return a validation-only document understood by the frozen Phase 5 admission."""
    value = copy.deepcopy(document)
    for function in value.get("functions", []):
        for block in function.get("body", {}).get("blocks", []):
            for instruction in block.get("instructions", []):
                if instruction.get("op") == "binary" and instruction.get("operator") in SHIFT_OPERATORS:
                    instruction["operator"] = "add"
                elif instruction.get("op") == "convert":
                    instruction.clear()
                    instruction.update({
                        "id": 0,
                        "op": "const",
                        "dest_local_id": 0,
                        "type_id": 0,
                        "value": 0,
                    })
    # Repair conversion placeholders with the original destination metadata in a second pass.
    original_functions = document.get("functions", [])
    for function_index, function in enumerate(value.get("functions", [])):
        for block_index, block in enumerate(function.get("body", {}).get("blocks", [])):
            original_block = original_functions[function_index]["body"]["blocks"][block_index]
            for instruction_index, instruction in enumerate(block.get("instructions", [])):
                original = original_block["instructions"][instruction_index]
                if original.get("op") == "convert":
                    instruction["id"] = int(original["id"])
                    instruction["dest_local_id"] = int(original["dest_local_id"])
                    instruction["type_id"] = int(original["type_id"])
                    instruction["value"] = 0
    return value


__all__ = [
    "CONVERSIONS",
    "PROFILE",
    "SHIFT_OPERATORS",
    "compatibility_view",
    "integer_bounds",
    "integer_type",
    "validate_machine_integer_semantics",
]
