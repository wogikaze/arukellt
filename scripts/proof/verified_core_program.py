"""Strict structural validator for VerifiedCore v1 program bodies.

The representation is intentionally small and fail-closed. Later proof phases
extend body semantics without changing the public ``validate_document`` API.
"""

from __future__ import annotations

from typing import Any

SCHEMA = "arukellt-verified-core"
VERSION = 1
WASM_TYPES = {"i32", "i64", "f32", "f64", "v128", "funcref", "externref"}
TYPE_KINDS = {"unit", "bool", "integer", "float", "string", "reference", "tuple", "struct", "enum", "function"}
VALUE_KINDS = {"local", "constant"}
EXPRESSION_KINDS = {"local", "result", "constant", "neg", "not", "add", "sub", "mul", "div", "mod", "eq", "ne", "lt", "le", "gt", "ge", "and", "or", "implies"}
INSTRUCTION_OPS = {"const", "copy", "unary", "binary", "call"}
TERMINATORS = {"return", "goto", "branch"}


def _fail(path: str, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def _obj(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        _fail(path, "expected object")
    return value


def _arr(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        _fail(path, "expected array")
    return value


def _string(value: Any, path: str) -> str:
    if not isinstance(value, str) or not value:
        _fail(path, "expected non-empty string")
    return value


def _integer(value: Any, path: str, *, minimum: int = 0) -> int:
    if type(value) is not int or value < minimum:
        _fail(path, f"expected integer >= {minimum}")
    return int(value)


def _exact(value: dict[str, Any], path: str, required: set[str], optional: set[str] = set()) -> None:
    missing = required - set(value)
    extra = set(value) - required - optional
    if missing:
        _fail(path, f"missing fields: {sorted(missing)}")
    if extra:
        _fail(path, f"unsupported fields: {sorted(extra)}")


def _type_ref(value: Any, path: str, type_ids: set[int]) -> int:
    type_id = _integer(value, path)
    if type_id not in type_ids:
        _fail(path, f"unknown type id {type_id}")
    return type_id


def _representation(value: Any, path: str) -> None:
    rep = _obj(value, path)
    _exact(rep, path, {"wasm", "nullable", "size_bytes", "align_bytes"}, {"layout_id"})
    for index, raw in enumerate(_arr(rep["wasm"], f"{path}.wasm")):
        wasm = _string(raw, f"{path}.wasm[{index}]")
        if wasm not in WASM_TYPES:
            _fail(f"{path}.wasm[{index}]", f"unsupported Wasm type {wasm}")
    if type(rep["nullable"]) is not bool:
        _fail(f"{path}.nullable", "expected boolean")
    _integer(rep["size_bytes"], f"{path}.size_bytes")
    align = _integer(rep["align_bytes"], f"{path}.align_bytes", minimum=1)
    if align & (align - 1):
        _fail(f"{path}.align_bytes", "must be a power of two")
    if "layout_id" in rep:
        _integer(rep["layout_id"], f"{path}.layout_id")


def _validate_types(raw_types: Any) -> tuple[list[dict[str, Any]], set[int]]:
    types = _arr(raw_types, "$.types")
    ids: set[int] = set()
    for index, raw in enumerate(types):
        path = f"$.types[{index}]"
        entry = _obj(raw, path)
        _exact(entry, path, {"id", "kind", "name", "representation"}, {"bits", "signed", "fields", "variants", "elements", "pointee_type_id"})
        type_id = _integer(entry["id"], f"{path}.id")
        if type_id in ids:
            _fail(f"{path}.id", f"duplicate type id {type_id}")
        ids.add(type_id)
        kind = _string(entry["kind"], f"{path}.kind")
        if kind not in TYPE_KINDS:
            _fail(f"{path}.kind", f"unsupported type kind {kind}")
        name = _string(entry["name"], f"{path}.name")
        if name == "unknown" or name.startswith("?"):
            _fail(f"{path}.name", "type identity must be explicit")
        _representation(entry["representation"], f"{path}.representation")
    for index, entry in enumerate(types):
        path = f"$.types[{index}]"
        kind = entry["kind"]
        if kind in {"integer", "float"}:
            _integer(entry.get("bits"), f"{path}.bits", minimum=1)
        if kind == "integer" and type(entry.get("signed")) is not bool:
            _fail(f"{path}.signed", "integer requires signed boolean")
        if kind == "reference":
            _type_ref(entry.get("pointee_type_id"), f"{path}.pointee_type_id", ids)
        for field in entry.get("fields", []):
            _type_ref(field.get("type_id"), f"{path}.fields.type_id", ids)
        for element in entry.get("elements", []):
            _type_ref(element, f"{path}.elements", ids)
        for variant in entry.get("variants", []):
            for payload in variant.get("payload_type_ids", []):
                _type_ref(payload, f"{path}.variants.payload_type_ids", ids)
    return types, ids


def _typed_value(value: Any, path: str, type_ids: set[int], local_types: dict[int, int]) -> int:
    node = _obj(value, path)
    _exact(node, path, {"kind", "type_id"}, {"local_id", "value"})
    kind = _string(node["kind"], f"{path}.kind")
    if kind not in VALUE_KINDS:
        _fail(f"{path}.kind", f"unsupported value kind {kind}")
    type_id = _type_ref(node["type_id"], f"{path}.type_id", type_ids)
    if kind == "local":
        local_id = _integer(node.get("local_id"), f"{path}.local_id")
        if local_types.get(local_id) != type_id:
            _fail(f"{path}.local_id", "unknown local or mismatched type")
    elif "value" not in node:
        _fail(path, "constant requires value")
    return type_id


def _expression(value: Any, path: str, type_ids: set[int], local_types: dict[int, int]) -> None:
    expr = _obj(value, path)
    _exact(expr, path, {"id", "kind", "type_id"}, {"operands", "local_id", "value"})
    _integer(expr["id"], f"{path}.id")
    kind = _string(expr["kind"], f"{path}.kind")
    if kind not in EXPRESSION_KINDS:
        _fail(f"{path}.kind", f"unsupported proof expression {kind}")
    type_id = _type_ref(expr["type_id"], f"{path}.type_id", type_ids)
    if kind == "local":
        local_id = _integer(expr.get("local_id"), f"{path}.local_id")
        if local_types.get(local_id) != type_id:
            _fail(f"{path}.local_id", "unknown local or mismatched type")
    if kind == "constant" and "value" not in expr:
        _fail(path, "constant requires value")
    for index, operand in enumerate(_arr(expr.get("operands", []), f"{path}.operands")):
        _expression(operand, f"{path}.operands[{index}]", type_ids, local_types)


def _edge_values(values: Any, path: str, type_ids: set[int], local_types: dict[int, int]) -> list[int]:
    return [_typed_value(raw, f"{path}[{index}]", type_ids, local_types) for index, raw in enumerate(_arr(values, path))]


def _validate_document_shape(value: Any) -> tuple[dict[str, Any], set[int], dict[int, dict[str, Any]]]:
    document = _obj(value, "$")
    _exact(document, "$", {"schema", "schema_version", "module", "target_profile", "types", "functions"}, {"generator"})
    if document["schema"] != SCHEMA or document["schema_version"] != VERSION:
        _fail("$", "unsupported VerifiedCore schema/version")
    _string(document["module"], "$.module")
    profile = _obj(document["target_profile"], "$.target_profile")
    _exact(profile, "$.target_profile", {"integer_model", "overflow", "floating_point", "pointer_width"})
    for key in ("integer_model", "overflow", "floating_point"):
        _string(profile[key], f"$.target_profile.{key}")
    if profile["pointer_width"] not in {32, 64}:
        _fail("$.target_profile.pointer_width", "expected 32 or 64")
    _, type_ids = _validate_types(document["types"])

    functions = _arr(document["functions"], "$.functions")
    function_by_id: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(functions):
        function = _obj(raw, f"$.functions[{index}]")
        function_id = _integer(function.get("id"), f"$.functions[{index}].id")
        if function_id in function_by_id:
            _fail(f"$.functions[{index}].id", f"duplicate function id {function_id}")
        function_by_id[function_id] = function
    return document, type_ids, function_by_id


def validate_document(value: Any) -> dict[str, Any]:
    document, type_ids, function_by_id = _validate_document_shape(value)
    for function_index, function in enumerate(document["functions"]):
        path = f"$.functions[{function_index}]"
        _exact(function, path, {"id", "name", "signature", "abi", "locals", "contracts", "body"})
        _string(function["name"], f"{path}.name")
        signature = _obj(function["signature"], f"{path}.signature")
        _exact(signature, f"{path}.signature", {"parameters", "return_type_id"})
        parameter_types: list[int] = []
        for parameter_index, raw in enumerate(_arr(signature["parameters"], f"{path}.signature.parameters")):
            parameter = _obj(raw, f"{path}.signature.parameters[{parameter_index}]")
            _exact(parameter, f"{path}.signature.parameters[{parameter_index}]", {"name", "type_id"})
            _string(parameter["name"], f"{path}.signature.parameters[{parameter_index}].name")
            parameter_types.append(_type_ref(parameter["type_id"], f"{path}.signature.parameters[{parameter_index}].type_id", type_ids))
        return_type = _type_ref(signature["return_type_id"], f"{path}.signature.return_type_id", type_ids)

        abi = _obj(function["abi"], f"{path}.abi")
        _exact(abi, f"{path}.abi", {"calling_convention", "parameters", "results"})
        _string(abi["calling_convention"], f"{path}.abi.calling_convention")
        if len(_arr(abi["parameters"], f"{path}.abi.parameters")) != len(parameter_types):
            _fail(f"{path}.abi.parameters", "count must match signature")

        locals_raw = _arr(function["locals"], f"{path}.locals")
        local_types: dict[int, int] = {}
        for local_index, raw in enumerate(locals_raw):
            local_path = f"{path}.locals[{local_index}]"
            local = _obj(raw, local_path)
            _exact(local, local_path, {"id", "name", "type_id", "storage"})
            local_id = _integer(local["id"], f"{local_path}.id")
            if local_id in local_types:
                _fail(f"{local_path}.id", f"duplicate local id {local_id}")
            local_types[local_id] = _type_ref(local["type_id"], f"{local_path}.type_id", type_ids)
            _string(local["name"], f"{local_path}.name")
            _string(local["storage"], f"{local_path}.storage")

        for contract_index, raw in enumerate(_arr(function["contracts"], f"{path}.contracts")):
            contract_path = f"{path}.contracts[{contract_index}]"
            contract = _obj(raw, contract_path)
            _exact(contract, contract_path, {"kind", "expression"}, {"result_name"})
            kind = _string(contract["kind"], f"{contract_path}.kind")
            if kind not in {"requires", "ensures", "invariant", "assert", "decreases"}:
                _fail(f"{contract_path}.kind", f"unsupported contract {kind}")
            if kind == "ensures":
                _string(contract.get("result_name"), f"{contract_path}.result_name")
            _expression(contract["expression"], f"{contract_path}.expression", type_ids, local_types)

        body = _obj(function["body"], f"{path}.body")
        _exact(body, f"{path}.body", {"entry_block", "blocks"})
        blocks = _arr(body["blocks"], f"{path}.body.blocks")
        block_by_id: dict[int, dict[str, Any]] = {}
        for block_index, raw in enumerate(blocks):
            block = _obj(raw, f"{path}.body.blocks[{block_index}]")
            block_id = _integer(block.get("id"), f"{path}.body.blocks[{block_index}].id")
            if block_id in block_by_id:
                _fail(f"{path}.body.blocks[{block_index}].id", f"duplicate block id {block_id}")
            block_by_id[block_id] = block
        entry = _integer(body["entry_block"], f"{path}.body.entry_block")
        if entry not in block_by_id:
            _fail(f"{path}.body.entry_block", f"unknown block {entry}")

        for block_index, block in enumerate(blocks):
            block_path = f"{path}.body.blocks[{block_index}]"
            _exact(block, block_path, {"id", "parameters", "instructions", "terminator"})
            for parameter_index, raw_parameter in enumerate(_arr(block["parameters"], f"{block_path}.parameters")):
                parameter_path = f"{block_path}.parameters[{parameter_index}]"
                parameter = _obj(raw_parameter, parameter_path)
                _exact(parameter, parameter_path, {"local_id", "type_id"})
                local_id = _integer(parameter["local_id"], f"{parameter_path}.local_id")
                type_id = _type_ref(parameter["type_id"], f"{parameter_path}.type_id", type_ids)
                if local_types.get(local_id) != type_id:
                    _fail(parameter_path, "block parameter must reference matching local")

            for instruction_index, raw_instruction in enumerate(_arr(block["instructions"], f"{block_path}.instructions")):
                instruction_path = f"{block_path}.instructions[{instruction_index}]"
                instruction = _obj(raw_instruction, instruction_path)
                op = _string(instruction.get("op"), f"{instruction_path}.op")
                if op not in INSTRUCTION_OPS:
                    _fail(f"{instruction_path}.op", f"unsupported instruction {op}")
                base_required = {"id", "op", "dest_local_id", "type_id"}
                optional = {"value", "arguments", "operator", "callee_id"}
                _exact(instruction, instruction_path, base_required, optional)
                _integer(instruction["id"], f"{instruction_path}.id")
                dest = _integer(instruction["dest_local_id"], f"{instruction_path}.dest_local_id")
                type_id = _type_ref(instruction["type_id"], f"{instruction_path}.type_id", type_ids)
                if local_types.get(dest) != type_id:
                    _fail(f"{instruction_path}.dest_local_id", "destination local type mismatch")
                if op == "const":
                    if "value" not in instruction:
                        _fail(instruction_path, "const requires value")
                else:
                    _edge_values(instruction.get("arguments", []), f"{instruction_path}.arguments", type_ids, local_types)
                if op in {"unary", "binary"}:
                    _string(instruction.get("operator"), f"{instruction_path}.operator")
                if op == "call":
                    _integer(instruction.get("callee_id"), f"{instruction_path}.callee_id")

            terminator = _obj(block["terminator"], f"{block_path}.terminator")
            kind = _string(terminator.get("kind"), f"{block_path}.terminator.kind")
            if kind not in TERMINATORS:
                _fail(f"{block_path}.terminator.kind", f"unsupported terminator {kind}")
            if kind == "return":
                _exact(terminator, f"{block_path}.terminator", {"kind"}, {"value"})
                if return_type == 0:
                    if "value" in terminator:
                        _fail(f"{block_path}.terminator.value", "unit return must not carry value")
                else:
                    if "value" not in terminator:
                        _fail(f"{block_path}.terminator", "value return required")
                    if _typed_value(terminator["value"], f"{block_path}.terminator.value", type_ids, local_types) != return_type:
                        _fail(f"{block_path}.terminator.value", "return type mismatch")
            elif kind == "goto":
                _exact(terminator, f"{block_path}.terminator", {"kind", "target", "arguments"})
                target = _integer(terminator["target"], f"{block_path}.terminator.target")
                if target not in block_by_id:
                    _fail(f"{block_path}.terminator.target", f"unknown block {target}")
                _edge_values(terminator["arguments"], f"{block_path}.terminator.arguments", type_ids, local_types)
            else:
                _exact(terminator, f"{block_path}.terminator", {"kind", "condition", "then_target", "else_target", "then_arguments", "else_arguments"})
                _typed_value(terminator["condition"], f"{block_path}.terminator.condition", type_ids, local_types)
                for key in ("then_target", "else_target"):
                    target = _integer(terminator[key], f"{block_path}.terminator.{key}")
                    if target not in block_by_id:
                        _fail(f"{block_path}.terminator.{key}", f"unknown block {target}")
                _edge_values(terminator["then_arguments"], f"{block_path}.terminator.then_arguments", type_ids, local_types)
                _edge_values(terminator["else_arguments"], f"{block_path}.terminator.else_arguments", type_ids, local_types)
    return document


__all__ = ["SCHEMA", "VERSION", "validate_document"]
