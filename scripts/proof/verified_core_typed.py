"""Independent semantic type validation for VerifiedCore v1."""

from __future__ import annotations

from typing import Any

from proof.verified_core import validate_document

SCHEMA = "arukellt-typed-verified-core-admission"
VERSION = 2


class TypedVerifiedCoreError(ValueError):
    pass


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise TypedVerifiedCoreError(message)


def _type_kinds(document: dict[str, Any]) -> dict[int, str]:
    return {int(entry["id"]): str(entry["kind"]) for entry in document["types"]}


def _check_constant(value: object, type_id: int, kinds: dict[int, str], path: str) -> None:
    kind = kinds[type_id]
    if kind == "bool":
        _require(type(value) is bool, f"{path}: bool constant requires boolean")
    elif kind == "integer":
        _require(type(value) is int, f"{path}: integer constant requires integer")
    else:
        raise TypedVerifiedCoreError(f"{path}: constants of type {kind!r} are outside proof phases 0-3")


def _value_type(value: dict[str, Any], local_types: dict[int, int], kinds: dict[int, str], path: str) -> int:
    type_id = int(value["type_id"])
    if value["kind"] == "local":
        _require(local_types.get(int(value["local_id"])) == type_id, f"{path}.local_id: local type mismatch")
    else:
        _check_constant(value["value"], type_id, kinds, f"{path}.value")
    return type_id


def _expression_type(expression: dict[str, Any], *, path: str, kinds: dict[int, str], local_types: dict[int, int], return_type: int, allow_result: bool, seen_ids: set[int]) -> int:
    expression_id = int(expression["id"])
    _require(expression_id not in seen_ids, f"{path}.id: duplicate expression id {expression_id}")
    seen_ids.add(expression_id)
    kind = str(expression["kind"])
    type_id = int(expression["type_id"])
    operands = expression.get("operands", [])
    if kind == "local":
        _require(not operands and local_types.get(int(expression["local_id"])) == type_id, f"{path}: local type mismatch")
        return type_id
    if kind == "result":
        _require(allow_result and return_type != 0 and type_id == return_type and not operands, f"{path}: invalid result expression")
        return type_id
    if kind == "constant":
        _require(not operands, f"{path}.operands: constant must be leaf")
        _check_constant(expression["value"], type_id, kinds, f"{path}.value")
        return type_id
    unary = {"neg", "not"}
    binary = {"add", "sub", "mul", "div", "mod", "eq", "ne", "lt", "le", "gt", "ge", "and", "or", "implies"}
    _require(kind in unary | binary, f"{path}.kind: unsupported expression {kind!r}")
    expected = 1 if kind in unary else 2
    _require(len(operands) == expected, f"{path}.operands: wrong arity")
    operand_types = [_expression_type(operand, path=f"{path}.operands[{index}]", kinds=kinds, local_types=local_types, return_type=return_type, allow_result=allow_result, seen_ids=seen_ids) for index, operand in enumerate(operands)]
    if kind == "neg":
        _require(kinds[operand_types[0]] == "integer" and type_id == operand_types[0], f"{path}: neg type mismatch")
    elif kind == "not":
        _require(kinds[operand_types[0]] == "bool" and kinds[type_id] == "bool", f"{path}: not type mismatch")
    else:
        left, right = operand_types
        _require(left == right, f"{path}.operands: TypeId mismatch")
        operand_kind = kinds[left]
        if kind in {"add", "sub", "mul", "div", "mod"}:
            _require(operand_kind == "integer" and type_id == left, f"{path}: arithmetic type mismatch")
        elif kind in {"lt", "le", "gt", "ge"}:
            _require(operand_kind == "integer" and kinds[type_id] == "bool", f"{path}: comparison type mismatch")
        elif kind in {"eq", "ne"}:
            _require(operand_kind in {"integer", "bool"} and kinds[type_id] == "bool", f"{path}: equality type mismatch")
        else:
            _require(operand_kind == "bool" and kinds[type_id] == "bool", f"{path}: logical type mismatch")
    return type_id


def _parameter_locals(function: dict[str, Any], path: str) -> dict[int, int]:
    local_types = {int(local["id"]): int(local["type_id"]) for local in function["locals"]}
    by_name = {str(local["name"]): local for local in function["locals"]}
    parameter_names = {str(parameter["name"]) for parameter in function["signature"]["parameters"]}
    for index, parameter in enumerate(function["signature"]["parameters"]):
        name = str(parameter["name"])
        local = by_name.get(name)
        _require(local is not None, f"{path}.signature.parameters[{index}]: missing parameter local")
        _require(local["storage"] == "parameter" and int(local["type_id"]) == int(parameter["type_id"]), f"{path}.locals: parameter {name!r} mismatch")
    for index, local in enumerate(function["locals"]):
        if local["storage"] == "parameter":
            _require(str(local["name"]) in parameter_names, f"{path}.locals[{index}]: undeclared parameter local")
    return local_types


def _check_instruction(instruction: dict[str, Any], *, path: str, local_types: dict[int, int], kinds: dict[int, str], functions: dict[int, dict[str, Any]]) -> None:
    op = str(instruction["op"])
    dest_type = int(instruction["type_id"])
    _require(local_types[int(instruction["dest_local_id"])] == dest_type, f"{path}.dest_local_id: type mismatch")
    if op == "const":
        _check_constant(instruction["value"], dest_type, kinds, f"{path}.value")
        return
    arg_types = [_value_type(value, local_types, kinds, f"{path}.arguments[{index}]") for index, value in enumerate(instruction.get("arguments", []))]
    if op == "copy":
        _require(len(arg_types) == 1 and arg_types[0] == dest_type, f"{path}: copy mismatch")
        return
    if op == "unary":
        operator = str(instruction["operator"])
        _require(len(arg_types) == 1, f"{path}: unary arity")
        if operator == "neg":
            _require(kinds[arg_types[0]] == "integer" and arg_types[0] == dest_type, f"{path}: neg mismatch")
        elif operator == "not":
            _require(kinds[arg_types[0]] == "bool" and kinds[dest_type] == "bool", f"{path}: not mismatch")
        else:
            raise TypedVerifiedCoreError(f"{path}.operator: unsupported unary {operator!r}")
        return
    if op == "binary":
        operator = str(instruction["operator"])
        _require(len(arg_types) == 2 and arg_types[0] == arg_types[1], f"{path}: binary TypeId mismatch")
        operand_type = arg_types[0]
        if operator in {"add", "sub", "mul", "div", "mod"}:
            _require(kinds[operand_type] == "integer" and dest_type == operand_type, f"{path}: arithmetic mismatch")
        elif operator in {"lt", "le", "gt", "ge"}:
            _require(kinds[operand_type] == "integer" and kinds[dest_type] == "bool", f"{path}: comparison mismatch")
        elif operator in {"eq", "ne"}:
            _require(kinds[operand_type] in {"integer", "bool"} and kinds[dest_type] == "bool", f"{path}: equality mismatch")
        elif operator in {"and", "or", "implies"}:
            _require(kinds[operand_type] == "bool" and kinds[dest_type] == "bool", f"{path}: logical mismatch")
        else:
            raise TypedVerifiedCoreError(f"{path}.operator: unsupported binary {operator!r}")
        return
    if op == "call":
        callee_id = int(instruction["callee_id"])
        _require(callee_id in functions, f"{path}.callee_id: unknown function")
        callee = functions[callee_id]
        expected = [int(parameter["type_id"]) for parameter in callee["signature"]["parameters"]]
        _require(arg_types == expected, f"{path}.arguments: call signature mismatch")
        callee_return = int(callee["signature"]["return_type_id"])
        _require(callee_return != 0 and callee_return == dest_type, f"{path}.type_id: call result mismatch")
        _require(any(contract["kind"] == "ensures" for contract in callee["contracts"]), f"{path}: called proof function requires ensures")
        return
    raise TypedVerifiedCoreError(f"{path}.op: unsupported instruction {op!r}")


def _successors(block: dict[str, Any]) -> list[int]:
    term = block["terminator"]
    if term["kind"] == "goto":
        return [int(term["target"])]
    if term["kind"] == "branch":
        return [int(term["then_target"]), int(term["else_target"])]
    return []


def _check_acyclic(blocks: dict[int, dict[str, Any]], path: str) -> None:
    visiting: set[int] = set()
    complete: set[int] = set()
    def visit(block_id: int) -> None:
        if block_id in complete:
            return
        if block_id in visiting:
            raise TypedVerifiedCoreError(f"{path}: cyclic CFG is outside proof phase 2")
        visiting.add(block_id)
        for target in _successors(blocks[block_id]):
            visit(target)
        visiting.remove(block_id)
        complete.add(block_id)
    for block_id in blocks:
        visit(block_id)


def _check_edges(function: dict[str, Any], path: str, local_types: dict[int, int], kinds: dict[int, str]) -> None:
    blocks = {int(block["id"]): block for block in function["body"]["blocks"]}
    _check_acyclic(blocks, f"{path}.body.blocks")
    parameter_types = {block_id: [int(parameter["type_id"]) for parameter in block["parameters"]] for block_id, block in blocks.items()}
    for index, block in enumerate(function["body"]["blocks"]):
        term = block["terminator"]
        term_path = f"{path}.body.blocks[{index}].terminator"
        if term["kind"] == "goto":
            actual = [_value_type(value, local_types, kinds, f"{term_path}.arguments[{i}]") for i, value in enumerate(term["arguments"])]
            _require(actual == parameter_types[int(term["target"])], f"{term_path}.arguments: block parameter mismatch")
        elif term["kind"] == "branch":
            condition_type = _value_type(term["condition"], local_types, kinds, f"{term_path}.condition")
            _require(kinds[condition_type] == "bool", f"{term_path}.condition: branch condition must be bool")
            for side in ("then", "else"):
                actual = [_value_type(value, local_types, kinds, f"{term_path}.{side}_arguments[{i}]") for i, value in enumerate(term[f"{side}_arguments"])]
                _require(actual == parameter_types[int(term[f"{side}_target"])], f"{term_path}.{side}_arguments: block parameter mismatch")


def _check_call_graph(functions: dict[int, dict[str, Any]]) -> None:
    edges = {function_id: {int(instruction["callee_id"]) for block in function["body"]["blocks"] for instruction in block["instructions"] if instruction["op"] == "call"} for function_id, function in functions.items()}
    visiting: set[int] = set()
    complete: set[int] = set()
    def visit(function_id: int) -> None:
        if function_id in complete:
            return
        if function_id in visiting:
            raise TypedVerifiedCoreError("$.functions: recursive proof calls are outside phase 3")
        visiting.add(function_id)
        for callee in edges[function_id]:
            visit(callee)
        visiting.remove(function_id)
        complete.add(function_id)
    for function_id in functions:
        visit(function_id)


def validate_typed_document(value: Any) -> dict[str, Any]:
    document = validate_document(value)
    kinds = _type_kinds(document)
    functions = {int(function["id"]): function for function in document["functions"]}
    for function_index, function in enumerate(document["functions"]):
        path = f"$.functions[{function_index}]"
        return_type = int(function["signature"]["return_type_id"])
        local_types = _parameter_locals(function, path)
        expression_ids: set[int] = set()
        for contract_index, contract in enumerate(function["contracts"]):
            contract_path = f"{path}.contracts[{contract_index}]"
            expression_type = _expression_type(contract["expression"], path=f"{contract_path}.expression", kinds=kinds, local_types=local_types, return_type=return_type, allow_result=contract["kind"] == "ensures", seen_ids=expression_ids)
            if contract["kind"] == "decreases":
                _require(kinds[expression_type] == "integer", f"{contract_path}.expression: decreases must be integer")
            else:
                _require(kinds[expression_type] == "bool", f"{contract_path}.expression: contract must be bool")
        for block_index, block in enumerate(function["body"]["blocks"]):
            for instruction_index, instruction in enumerate(block["instructions"]):
                _check_instruction(instruction, path=f"{path}.body.blocks[{block_index}].instructions[{instruction_index}]", local_types=local_types, kinds=kinds, functions=functions)
            terminator = block["terminator"]
            if terminator["kind"] == "return" and "value" in terminator:
                _require(_value_type(terminator["value"], local_types, kinds, f"{path}.body.blocks[{block_index}].terminator.value") == return_type, f"{path}.body.blocks[{block_index}].terminator.value: return type mismatch")
        _check_edges(function, path, local_types, kinds)
    _check_call_graph(functions)
    return document


__all__ = ["SCHEMA", "VERSION", "TypedVerifiedCoreError", "validate_typed_document"]
