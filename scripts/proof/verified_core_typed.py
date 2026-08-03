"""Independent semantic type validation for VerifiedCore v1.

The structural validator proves schema/reference integrity. This validator proves
that every accepted expression, contract, parameter local, and return value is
well typed without reconstructing types from names or operator text.
"""

from __future__ import annotations

from typing import Any

from proof.verified_core import validate_document

SCHEMA = "arukellt-typed-verified-core-admission"
VERSION = 1


class TypedVerifiedCoreError(ValueError):
    """VerifiedCore is structurally valid but violates typed semantics."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise TypedVerifiedCoreError(message)


def _type_kinds(document: dict[str, Any]) -> dict[int, str]:
    return {int(entry["id"]): str(entry["kind"]) for entry in document["types"]}


def _check_constant(value: object, type_id: int, kinds: dict[int, str], path: str) -> None:
    kind = kinds[type_id]
    if kind == "bool":
        _require(type(value) is bool, f"{path}: bool constant requires a boolean value")
        return
    if kind == "integer":
        _require(type(value) is int, f"{path}: integer constant requires an integer value")
        return
    raise TypedVerifiedCoreError(
        f"{path}: constants of type kind {kind!r} are outside typed VerifiedCore v1"
    )


def _expression_type(
    expression: dict[str, Any],
    *,
    path: str,
    kinds: dict[int, str],
    local_types: dict[int, int],
    return_type: int,
    allow_result: bool,
    seen_ids: set[int],
) -> int:
    expression_id = int(expression["id"])
    if expression_id in seen_ids:
        raise TypedVerifiedCoreError(f"{path}.id: duplicate expression id {expression_id}")
    seen_ids.add(expression_id)

    kind = str(expression["kind"])
    type_id = int(expression["type_id"])
    operands = expression.get("operands", [])
    _require(isinstance(operands, list), f"{path}.operands: expected array")

    if kind == "local":
        _require(set(expression) <= {"id", "kind", "type_id", "local_id", "operands"},
                 f"{path}: local expression has unsupported fields")
        _require(not operands, f"{path}.operands: local expression must be a leaf")
        local_id = expression.get("local_id")
        _require(type(local_id) is int and local_id in local_types,
                 f"{path}.local_id: unknown local {local_id!r}")
        _require(local_types[int(local_id)] == type_id,
                 f"{path}.type_id: local expression type does not match declaration")
        return type_id

    if kind == "result":
        _require(set(expression) <= {"id", "kind", "type_id", "operands"},
                 f"{path}: result expression has unsupported fields")
        _require(allow_result, f"{path}: result is only valid in ensures contracts")
        _require(not operands, f"{path}.operands: result expression must be a leaf")
        _require(return_type != 0, f"{path}: unit-returning function has no result value")
        _require(type_id == return_type,
                 f"{path}.type_id: result type does not match function return type")
        return type_id

    if kind == "constant":
        _require(set(expression) <= {"id", "kind", "type_id", "value", "operands"},
                 f"{path}: constant expression has unsupported fields")
        _require(not operands, f"{path}.operands: constant expression must be a leaf")
        _require("value" in expression, f"{path}.value: constant value is required")
        _check_constant(expression["value"], type_id, kinds, f"{path}.value")
        return type_id

    unary = {"neg", "not"}
    binary = {"add", "sub", "mul", "div", "mod", "eq", "ne", "lt", "le", "gt", "ge", "and", "or"}
    if kind not in unary | binary:
        raise TypedVerifiedCoreError(
            f"{path}.kind: unsupported typed expression kind {kind!r}"
        )
    _require(set(expression) <= {"id", "kind", "type_id", "operands"},
             f"{path}: operator expression has unsupported fields")
    expected_arity = 1 if kind in unary else 2
    _require(len(operands) == expected_arity,
             f"{path}.operands: {kind} requires {expected_arity} operand(s)")
    operand_types = [
        _expression_type(
            operand,
            path=f"{path}.operands[{index}]",
            kinds=kinds,
            local_types=local_types,
            return_type=return_type,
            allow_result=allow_result,
            seen_ids=seen_ids,
        )
        for index, operand in enumerate(operands)
    ]

    if kind == "neg":
        _require(kinds[operand_types[0]] == "integer", f"{path}: neg operand must be integer")
        _require(type_id == operand_types[0], f"{path}.type_id: neg must preserve operand type")
        return type_id
    if kind == "not":
        _require(kinds[operand_types[0]] == "bool", f"{path}: not operand must be bool")
        _require(kinds[type_id] == "bool", f"{path}.type_id: not result must be bool")
        return type_id

    left, right = operand_types
    _require(left == right, f"{path}.operands: binary operands must have identical TypeId")
    operand_kind = kinds[left]
    if kind in {"add", "sub", "mul", "div", "mod"}:
        _require(operand_kind == "integer", f"{path}: arithmetic operands must be integer")
        _require(type_id == left, f"{path}.type_id: arithmetic must preserve operand TypeId")
    elif kind in {"lt", "le", "gt", "ge"}:
        _require(operand_kind == "integer", f"{path}: ordered comparison operands must be integer")
        _require(kinds[type_id] == "bool", f"{path}.type_id: comparison result must be bool")
    elif kind in {"eq", "ne"}:
        _require(operand_kind in {"integer", "bool"},
                 f"{path}: equality operands must be integer or bool")
        _require(kinds[type_id] == "bool", f"{path}.type_id: equality result must be bool")
    else:
        _require(operand_kind == "bool", f"{path}: logical operands must be bool")
        _require(kinds[type_id] == "bool", f"{path}.type_id: logical result must be bool")
    return type_id


def _validate_parameter_locals(function: dict[str, Any], path: str) -> dict[int, int]:
    signature_parameters = function["signature"]["parameters"]
    locals_raw = function["locals"]
    local_types: dict[int, int] = {}
    local_names: dict[str, dict[str, Any]] = {}
    for index, local in enumerate(locals_raw):
        name = str(local["name"])
        if name in local_names:
            raise TypedVerifiedCoreError(f"{path}.locals[{index}].name: duplicate local {name!r}")
        local_names[name] = local
        local_types[int(local["id"])] = int(local["type_id"])

    parameter_names = {str(parameter["name"]) for parameter in signature_parameters}
    for index, parameter in enumerate(signature_parameters):
        name = str(parameter["name"])
        local = local_names.get(name)
        _require(local is not None,
                 f"{path}.signature.parameters[{index}]: no corresponding parameter local")
        _require(local["storage"] == "parameter",
                 f"{path}.locals: signature parameter {name!r} is not storage=parameter")
        _require(int(local["type_id"]) == int(parameter["type_id"]),
                 f"{path}.locals: parameter local {name!r} has a different TypeId")
    for index, local in enumerate(locals_raw):
        if local["storage"] == "parameter":
            _require(str(local["name"]) in parameter_names,
                     f"{path}.locals[{index}]: parameter local is absent from signature")
    return local_types


def _validate_return_value(
    value: dict[str, Any],
    *,
    path: str,
    return_type: int,
    kinds: dict[int, str],
    local_types: dict[int, int],
) -> None:
    type_id = int(value["type_id"])
    _require(type_id == return_type, f"{path}.type_id: return value type mismatch")
    kind = str(value["kind"])
    if kind == "local":
        local_id = int(value["local_id"])
        _require(local_types.get(local_id) == type_id,
                 f"{path}.local_id: return local type mismatch")
    elif kind == "constant":
        _check_constant(value["value"], type_id, kinds, f"{path}.value")
    else:
        raise TypedVerifiedCoreError(f"{path}.kind: unsupported return value kind {kind!r}")


def validate_typed_document(value: Any) -> dict[str, Any]:
    document = validate_document(value)
    kinds = _type_kinds(document)
    for function_index, function in enumerate(document["functions"]):
        path = f"$.functions[{function_index}]"
        return_type = int(function["signature"]["return_type_id"])
        local_types = _validate_parameter_locals(function, path)
        expression_ids: set[int] = set()
        for contract_index, contract in enumerate(function["contracts"]):
            contract_path = f"{path}.contracts[{contract_index}]"
            contract_kind = str(contract["kind"])
            expression_type = _expression_type(
                contract["expression"],
                path=f"{contract_path}.expression",
                kinds=kinds,
                local_types=local_types,
                return_type=return_type,
                allow_result=contract_kind == "ensures",
                seen_ids=expression_ids,
            )
            if contract_kind == "decreases":
                _require(kinds[expression_type] == "integer",
                         f"{contract_path}.expression: decreases must be integer")
            else:
                _require(kinds[expression_type] == "bool",
                         f"{contract_path}.expression: contract must have type bool")

        for block_index, block in enumerate(function["body"]["blocks"]):
            terminator = block["terminator"]
            if terminator["kind"] == "return" and "value" in terminator:
                _validate_return_value(
                    terminator["value"],
                    path=f"{path}.body.blocks[{block_index}].terminator.value",
                    return_type=return_type,
                    kinds=kinds,
                    local_types=local_types,
                )
    return document


__all__ = [
    "SCHEMA",
    "VERSION",
    "TypedVerifiedCoreError",
    "validate_typed_document",
]
