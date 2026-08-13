"""Independent fail-closed validator for typed VerifiedCore v1."""

from __future__ import annotations

from typing import Any

from proof.common import (
    array_value,
    bool_value,
    exact_keys,
    fail,
    int_value,
    object_value,
    string_value,
    unique_id_map,
    validate_header,
)

SCHEMA = "arukellt-verified-core"
VERSION = 1
WASM_TYPES = {"i32", "i64", "f32", "f64", "v128", "funcref", "externref"}
TYPE_KINDS = {
    "unit", "bool", "integer", "float", "string", "reference", "tuple",
    "struct", "enum", "function",
}


def _type_ref(value: Any, path: str, type_ids: set[int]) -> int:
    type_id = int_value(value, path, minimum=0)
    if type_id not in type_ids:
        fail(path, f"unknown type id: {type_id}")
    return type_id


def _representation(value: Any, path: str) -> None:
    rep = object_value(value, path)
    exact_keys(
        rep,
        path,
        required={"wasm", "nullable", "size_bytes", "align_bytes"},
        optional={"layout_id"},
    )
    for index, raw in enumerate(array_value(rep["wasm"], f"{path}.wasm")):
        wasm_type = string_value(raw, f"{path}.wasm[{index}]")
        if wasm_type not in WASM_TYPES:
            fail(f"{path}.wasm[{index}]", f"unsupported Wasm value type: {wasm_type}")
    bool_value(rep["nullable"], f"{path}.nullable")
    int_value(rep["size_bytes"], f"{path}.size_bytes", minimum=0)
    align = int_value(rep["align_bytes"], f"{path}.align_bytes", minimum=1)
    if align & (align - 1):
        fail(f"{path}.align_bytes", "must be a power of two")
    if "layout_id" in rep:
        int_value(rep["layout_id"], f"{path}.layout_id", minimum=0)


def _type(value: Any, path: str, type_ids: set[int]) -> None:
    ty = object_value(value, path)
    exact_keys(
        ty,
        path,
        required={"id", "kind", "name", "representation"},
        optional={"bits", "signed", "fields", "variants", "elements", "pointee_type_id"},
    )
    int_value(ty["id"], f"{path}.id", minimum=0)
    kind = string_value(ty["kind"], f"{path}.kind")
    if kind not in TYPE_KINDS:
        fail(f"{path}.kind", f"unsupported type kind: {kind}")
    name = string_value(ty["name"], f"{path}.name")
    if name == "unknown" or name.startswith("?"):
        fail(f"{path}.name", "type identity must be explicit")
    _representation(ty["representation"], f"{path}.representation")

    if kind in {"integer", "float"}:
        int_value(ty.get("bits"), f"{path}.bits", minimum=1)
    if kind == "integer":
        bool_value(ty.get("signed"), f"{path}.signed")
    if kind == "reference":
        _type_ref(ty.get("pointee_type_id"), f"{path}.pointee_type_id", type_ids)
    if kind == "tuple":
        for index, type_id in enumerate(array_value(ty.get("elements"), f"{path}.elements")):
            _type_ref(type_id, f"{path}.elements[{index}]", type_ids)
    if kind == "struct":
        seen_names: set[str] = set()
        for index, raw in enumerate(array_value(ty.get("fields"), f"{path}.fields")):
            field_path = f"{path}.fields[{index}]"
            field = object_value(raw, field_path)
            exact_keys(field, field_path, required={"name", "type_id", "offset_bytes"})
            field_name = string_value(field["name"], f"{field_path}.name")
            if field_name in seen_names:
                fail(f"{field_path}.name", f"duplicate field: {field_name}")
            seen_names.add(field_name)
            _type_ref(field["type_id"], f"{field_path}.type_id", type_ids)
            int_value(field["offset_bytes"], f"{field_path}.offset_bytes", minimum=0)
    if kind == "enum":
        seen_tags: set[int] = set()
        for index, raw in enumerate(array_value(ty.get("variants"), f"{path}.variants")):
            variant_path = f"{path}.variants[{index}]"
            variant = object_value(raw, variant_path)
            exact_keys(variant, variant_path, required={"name", "tag", "payload_type_ids"})
            string_value(variant["name"], f"{variant_path}.name")
            tag = int_value(variant["tag"], f"{variant_path}.tag", minimum=0)
            if tag in seen_tags:
                fail(f"{variant_path}.tag", f"duplicate tag: {tag}")
            seen_tags.add(tag)
            for payload_index, type_id in enumerate(
                array_value(variant["payload_type_ids"], f"{variant_path}.payload_type_ids")
            ):
                _type_ref(type_id, f"{variant_path}.payload_type_ids[{payload_index}]", type_ids)


def _typed_value(value: Any, path: str, type_ids: set[int], local_types: dict[int, int]) -> int:
    node = object_value(value, path)
    exact_keys(node, path, required={"kind", "type_id"}, optional={"local_id", "value"})
    kind = string_value(node["kind"], f"{path}.kind")
    type_id = _type_ref(node["type_id"], f"{path}.type_id", type_ids)
    if kind == "local":
        local_id = int_value(node.get("local_id"), f"{path}.local_id", minimum=0)
        if local_id not in local_types:
            fail(f"{path}.local_id", f"unknown local id: {local_id}")
        if local_types[local_id] != type_id:
            fail(f"{path}.type_id", "local value type must match local declaration")
    elif kind == "constant":
        if "value" not in node:
            fail(path, "constant requires value")
    else:
        fail(f"{path}.kind", f"unsupported typed value kind: {kind}")
    return type_id


def _expression(value: Any, path: str, type_ids: set[int], local_types: dict[int, int]) -> None:
    expr = object_value(value, path)
    exact_keys(
        expr,
        path,
        required={"id", "kind", "type_id"},
        optional={"operands", "local_id", "value", "callee_id", "field_id"},
    )
    int_value(expr["id"], f"{path}.id", minimum=0)
    string_value(expr["kind"], f"{path}.kind")
    type_id = _type_ref(expr["type_id"], f"{path}.type_id", type_ids)
    if "local_id" in expr:
        local_id = int_value(expr["local_id"], f"{path}.local_id", minimum=0)
        if local_id not in local_types:
            fail(f"{path}.local_id", f"unknown local id: {local_id}")
        if local_types[local_id] != type_id:
            fail(f"{path}.type_id", "expression type must match local declaration")
    if "callee_id" in expr:
        int_value(expr["callee_id"], f"{path}.callee_id", minimum=0)
    if "field_id" in expr:
        int_value(expr["field_id"], f"{path}.field_id", minimum=0)
    operands = array_value(expr.get("operands", []), f"{path}.operands")
    unique_id_map(operands, f"{path}.operands")
    for index, operand in enumerate(operands):
        _expression(operand, f"{path}.operands[{index}]", type_ids, local_types)


def _abi_item(value: Any, path: str, expected_type: int, type_ids: set[int]) -> None:
    item = object_value(value, path)
    exact_keys(item, path, required={"type_id", "passing", "wasm"})
    type_id = _type_ref(item["type_id"], f"{path}.type_id", type_ids)
    if type_id != expected_type:
        fail(f"{path}.type_id", "ABI type must match signature")
    string_value(item["passing"], f"{path}.passing")
    for index, raw in enumerate(array_value(item["wasm"], f"{path}.wasm")):
        wasm_type = string_value(raw, f"{path}.wasm[{index}]")
        if wasm_type not in WASM_TYPES:
            fail(f"{path}.wasm[{index}]", f"unsupported Wasm value type: {wasm_type}")


def _function(value: Any, path: str, type_ids: set[int]) -> None:
    function = object_value(value, path)
    exact_keys(
        function,
        path,
        required={"id", "name", "signature", "abi", "locals", "contracts", "body"},
    )
    int_value(function["id"], f"{path}.id", minimum=0)
    name = string_value(function["name"], f"{path}.name")
    if name == "<unknown>":
        fail(f"{path}.name", "function identity must be explicit")

    signature = object_value(function["signature"], f"{path}.signature")
    exact_keys(signature, f"{path}.signature", required={"parameters", "return_type_id"})
    parameter_types: list[int] = []
    seen_parameters: set[str] = set()
    for index, raw in enumerate(array_value(signature["parameters"], f"{path}.signature.parameters")):
        parameter_path = f"{path}.signature.parameters[{index}]"
        parameter = object_value(raw, parameter_path)
        exact_keys(parameter, parameter_path, required={"name", "type_id"})
        parameter_name = string_value(parameter["name"], f"{parameter_path}.name")
        if parameter_name in seen_parameters:
            fail(f"{parameter_path}.name", f"duplicate parameter: {parameter_name}")
        seen_parameters.add(parameter_name)
        parameter_types.append(_type_ref(parameter["type_id"], f"{parameter_path}.type_id", type_ids))
    return_type = _type_ref(signature["return_type_id"], f"{path}.signature.return_type_id", type_ids)

    abi = object_value(function["abi"], f"{path}.abi")
    exact_keys(abi, f"{path}.abi", required={"calling_convention", "parameters", "results"})
    string_value(abi["calling_convention"], f"{path}.abi.calling_convention")
    abi_parameters = array_value(abi["parameters"], f"{path}.abi.parameters")
    if len(abi_parameters) != len(parameter_types):
        fail(f"{path}.abi.parameters", "ABI parameter count must match signature")
    for index, item in enumerate(abi_parameters):
        _abi_item(item, f"{path}.abi.parameters[{index}]", parameter_types[index], type_ids)
    abi_results = array_value(abi["results"], f"{path}.abi.results")
    expected_result_count = 0 if return_type == 0 else 1
    if len(abi_results) != expected_result_count:
        fail(f"{path}.abi.results", "ABI result count must match signature")
    if abi_results:
        _abi_item(abi_results[0], f"{path}.abi.results[0]", return_type, type_ids)

    locals_raw = array_value(function["locals"], f"{path}.locals")
    local_map = unique_id_map(locals_raw, f"{path}.locals")
    local_types: dict[int, int] = {}
    for index, raw in enumerate(locals_raw):
        local_path = f"{path}.locals[{index}]"
        local = object_value(raw, local_path)
        exact_keys(local, local_path, required={"id", "name", "type_id", "storage"})
        string_value(local["name"], f"{local_path}.name")
        local_types[local["id"]] = _type_ref(local["type_id"], f"{local_path}.type_id", type_ids)
        string_value(local["storage"], f"{local_path}.storage")

    for index, raw in enumerate(array_value(function["contracts"], f"{path}.contracts")):
        contract_path = f"{path}.contracts[{index}]"
        contract = object_value(raw, contract_path)
        exact_keys(contract, contract_path, required={"kind", "expression"}, optional={"result_name"})
        kind = string_value(contract["kind"], f"{contract_path}.kind")
        if kind not in {"requires", "ensures", "invariant", "assert", "decreases"}:
            fail(f"{contract_path}.kind", f"unsupported contract kind: {kind}")
        if kind == "ensures":
            string_value(contract.get("result_name"), f"{contract_path}.result_name")
        elif "result_name" in contract:
            fail(f"{contract_path}.result_name", "only valid for ensures")
        _expression(contract["expression"], f"{contract_path}.expression", type_ids, local_types)

    body = object_value(function["body"], f"{path}.body")
    exact_keys(body, f"{path}.body", required={"entry_block", "blocks"})
    blocks = array_value(body["blocks"], f"{path}.body.blocks")
    block_map = unique_id_map(blocks, f"{path}.body.blocks")
    entry = int_value(body["entry_block"], f"{path}.body.entry_block", minimum=0)
    if entry not in block_map:
        fail(f"{path}.body.entry_block", f"unknown block id: {entry}")
    for index, raw in enumerate(blocks):
        block_path = f"{path}.body.blocks[{index}]"
        block = object_value(raw, block_path)
        exact_keys(block, block_path, required={"id", "parameters", "instructions", "terminator"})
        array_value(block["parameters"], f"{block_path}.parameters")
        array_value(block["instructions"], f"{block_path}.instructions")
        terminator = object_value(block["terminator"], f"{block_path}.terminator")
        exact_keys(terminator, f"{block_path}.terminator", required={"kind"}, optional={"value", "target"})
        kind = string_value(terminator["kind"], f"{block_path}.terminator.kind")
        if kind == "return":
            if return_type == 0:
                if "value" in terminator:
                    fail(f"{block_path}.terminator.value", "unit return must not carry a value")
            else:
                if "value" not in terminator:
                    fail(f"{block_path}.terminator", "non-unit return requires value")
                actual = _typed_value(
                    terminator["value"], f"{block_path}.terminator.value", type_ids, local_types
                )
                if actual != return_type:
                    fail(f"{block_path}.terminator.value.type_id", "return type must match signature")
        elif kind == "goto":
            target = int_value(terminator.get("target"), f"{block_path}.terminator.target", minimum=0)
            if target not in block_map:
                fail(f"{block_path}.terminator.target", f"unknown block id: {target}")
        else:
            fail(f"{block_path}.terminator.kind", f"unsupported terminator: {kind}")


def validate_document(value: Any) -> dict[str, Any]:
    document = object_value(value, "$")
    exact_keys(
        document,
        "$",
        required={"schema", "schema_version", "module", "target_profile", "types", "functions"},
        optional={"generator"},
    )
    validate_header(document, "$", SCHEMA, VERSION)
    string_value(document["module"], "$.module")
    if "generator" in document:
        string_value(document["generator"], "$.generator")
    profile = object_value(document["target_profile"], "$.target_profile")
    exact_keys(
        profile,
        "$.target_profile",
        required={"integer_model", "overflow", "floating_point", "pointer_width"},
    )
    for field in ("integer_model", "overflow", "floating_point"):
        string_value(profile[field], f"$.target_profile.{field}")
    if int_value(profile["pointer_width"], "$.target_profile.pointer_width") not in {32, 64}:
        fail("$.target_profile.pointer_width", "expected 32 or 64")

    types = array_value(document["types"], "$.types")
    type_map = unique_id_map(types, "$.types")
    if 0 not in type_map or type_map[0].get("kind") != "unit":
        fail("$.types", "type id 0 must be unit")
    for index, ty in enumerate(types):
        _type(ty, f"$.types[{index}]", set(type_map))

    functions = array_value(document["functions"], "$.functions")
    unique_id_map(functions, "$.functions")
    seen_names: set[str] = set()
    for index, function in enumerate(functions):
        path = f"$.functions[{index}]"
        _function(function, path, set(type_map))
        name = function["name"]
        if name in seen_names:
            fail(f"{path}.name", f"duplicate function name: {name}")
        seen_names.add(name)
    return document
