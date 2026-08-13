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
WASM_TYPES = {"i32", "i64", "f32", "f64", "v128", "funcref", "externref", "gc-ref"}
VALUE_TYPES = {"void", "i32", "i64", "f32", "f64", "v128", "ref", "gc-ref", "funcref"}
TYPE_KINDS = {"unit", "bool", "integer", "float", "string", "reference", "function", "vector"}
REPRESENTATION_KINDS = {"unit", "scalar", "linear-ptr", "gc-ref", "funcref"}
CONTRACT_KINDS = {"requires", "ensures", "invariant", "assert", "decreases"}


def _type_ref(value: Any, path: str, type_ids: set[int]) -> int:
    type_id = int_value(value, path, minimum=0)
    if type_id not in type_ids:
        fail(path, f"unknown type id: {type_id}")
    return type_id


def _wasm_values(value: Any, path: str) -> list[str]:
    result: list[str] = []
    for index, raw in enumerate(array_value(value, path)):
        wasm_type = string_value(raw, f"{path}[{index}]")
        if wasm_type not in WASM_TYPES:
            fail(f"{path}[{index}]", f"unsupported Wasm value type: {wasm_type}")
        result.append(wasm_type)
    return result


def _representation(value: Any, path: str, value_type: str) -> None:
    rep = object_value(value, path)
    exact_keys(
        rep,
        path,
        required={"kind", "wasm", "nullable", "size_bytes", "align_bytes"},
        optional={"layout_id"},
    )
    kind = string_value(rep["kind"], f"{path}.kind")
    if kind not in REPRESENTATION_KINDS:
        fail(f"{path}.kind", f"unsupported representation kind: {kind}")
    wasm = _wasm_values(rep["wasm"], f"{path}.wasm")
    nullable = bool_value(rep["nullable"], f"{path}.nullable")
    size = int_value(rep["size_bytes"], f"{path}.size_bytes", minimum=0)
    align = int_value(rep["align_bytes"], f"{path}.align_bytes", minimum=1)
    if align & (align - 1):
        fail(f"{path}.align_bytes", "must be a power of two")
    if "layout_id" in rep:
        int_value(rep["layout_id"], f"{path}.layout_id", minimum=0)

    expected_wasm = {
        "void": [],
        "i32": ["i32"],
        "i64": ["i64"],
        "f32": ["f32"],
        "f64": ["f64"],
        "v128": ["v128"],
        "ref": ["i32"],
        "gc-ref": ["gc-ref"],
        "funcref": ["funcref"],
    }[value_type]
    if wasm != expected_wasm:
        fail(f"{path}.wasm", f"must match value_type={value_type}")
    if value_type == "void" and (kind != "unit" or nullable or size != 0):
        fail(path, "void representation must be non-null unit with zero size")
    if value_type in {"ref", "gc-ref", "funcref"} and not nullable:
        fail(f"{path}.nullable", "reference representation must state nullability explicitly")


def _type(value: Any, path: str) -> None:
    ty = object_value(value, path)
    exact_keys(ty, path, required={"id", "kind", "name", "value_type", "representation"})
    int_value(ty["id"], f"{path}.id", minimum=0)
    kind = string_value(ty["kind"], f"{path}.kind")
    if kind not in TYPE_KINDS:
        fail(f"{path}.kind", f"unsupported type kind: {kind}")
    name = string_value(ty["name"], f"{path}.name")
    if name == "unknown" or name.startswith("?"):
        fail(f"{path}.name", "type identity must be explicit")
    value_type = string_value(ty["value_type"], f"{path}.value_type")
    if value_type not in VALUE_TYPES:
        fail(f"{path}.value_type", f"unsupported value type: {value_type}")
    _representation(ty["representation"], f"{path}.representation", value_type)


def _abi_item(value: Any, path: str, expected_type: int, type_ids: set[int]) -> None:
    item = object_value(value, path)
    exact_keys(item, path, required={"type_id", "passing", "wasm"})
    type_id = _type_ref(item["type_id"], f"{path}.type_id", type_ids)
    if type_id != expected_type:
        fail(f"{path}.type_id", "ABI type must match signature")
    passing = string_value(item["passing"], f"{path}.passing")
    if passing not in {"value", "reference"}:
        fail(f"{path}.passing", f"unsupported passing mode: {passing}")
    _wasm_values(item["wasm"], f"{path}.wasm")


def _local(value: Any, path: str, type_ids: set[int]) -> tuple[int, int]:
    local = object_value(value, path)
    exact_keys(local, path, required={"id", "name", "type_id", "storage"})
    local_id = int_value(local["id"], f"{path}.id", minimum=0)
    string_value(local["name"], f"{path}.name")
    type_id = _type_ref(local["type_id"], f"{path}.type_id", type_ids)
    storage = string_value(local["storage"], f"{path}.storage")
    if storage not in {"parameter", "local", "temporary"}:
        fail(f"{path}.storage", f"unsupported storage kind: {storage}")
    return local_id, type_id


def _expression(value: Any, path: str, type_ids: set[int]) -> dict[str, Any]:
    expr = object_value(value, path)
    exact_keys(
        expr,
        path,
        required={
            "id",
            "kind",
            "kind_id",
            "type_id",
            "value_type",
            "text",
            "int_value",
            "float_value",
            "span_start",
            "children",
        },
    )
    int_value(expr["id"], f"{path}.id", minimum=0)
    string_value(expr["kind"], f"{path}.kind")
    int_value(expr["kind_id"], f"{path}.kind_id", minimum=0)
    _type_ref(expr["type_id"], f"{path}.type_id", type_ids)
    value_type = string_value(expr["value_type"], f"{path}.value_type")
    if value_type not in VALUE_TYPES:
        fail(f"{path}.value_type", f"unsupported value type: {value_type}")
    string_value(expr["text"], f"{path}.text", nonempty=False)
    int_value(expr["int_value"], f"{path}.int_value")
    if isinstance(expr["float_value"], bool) or not isinstance(expr["float_value"], (int, float)):
        fail(f"{path}.float_value", "expected number")
    int_value(expr["span_start"], f"{path}.span_start", minimum=0)
    for child_index, child in enumerate(array_value(expr["children"], f"{path}.children")):
        int_value(child, f"{path}.children[{child_index}]", minimum=0)
    return expr


def _function(value: Any, path: str, type_ids: set[int], type_wasm: dict[int, list[str]]) -> None:
    function = object_value(value, path)
    exact_keys(function, path, required={"id", "name", "signature", "abi", "locals", "contracts", "body"})
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
        if item["wasm"] != type_wasm[parameter_types[index]]:
            fail(f"{path}.abi.parameters[{index}].wasm", "ABI representation must match type table")
    abi_results = array_value(abi["results"], f"{path}.abi.results")
    expected_result_count = 0 if type_wasm[return_type] == [] else 1
    if len(abi_results) != expected_result_count:
        fail(f"{path}.abi.results", "ABI result count must match return representation")
    if abi_results:
        _abi_item(abi_results[0], f"{path}.abi.results[0]", return_type, type_ids)
        if abi_results[0]["wasm"] != type_wasm[return_type]:
            fail(f"{path}.abi.results[0].wasm", "ABI representation must match type table")

    locals_raw = array_value(function["locals"], f"{path}.locals")
    local_map = unique_id_map(locals_raw, f"{path}.locals")
    local_types: dict[int, int] = {}
    for index, raw in enumerate(locals_raw):
        local_id, type_id = _local(raw, f"{path}.locals[{index}]", type_ids)
        local_types[local_id] = type_id
    for index, parameter_type in enumerate(parameter_types):
        if index not in local_map:
            fail(f"{path}.locals", f"missing parameter local id: {index}")
        if local_types[index] != parameter_type or local_map[index]["storage"] != "parameter":
            fail(f"{path}.locals[{index}]", "parameter local must match signature")

    body = object_value(function["body"], f"{path}.body")
    exact_keys(body, f"{path}.body", required={"root_expr_id", "expressions"})
    expressions = array_value(body["expressions"], f"{path}.body.expressions")
    expression_map = unique_id_map(expressions, f"{path}.body.expressions")
    root = int_value(body["root_expr_id"], f"{path}.body.root_expr_id", minimum=0)
    if root not in expression_map:
        fail(f"{path}.body.root_expr_id", f"unknown expression id: {root}")
    for index, raw in enumerate(expressions):
        expr = _expression(raw, f"{path}.body.expressions[{index}]", type_ids)
        for child_index, child in enumerate(expr["children"]):
            if child not in expression_map:
                fail(
                    f"{path}.body.expressions[{index}].children[{child_index}]",
                    f"unknown expression id: {child}",
                )

    reachable: set[int] = set()
    stack = [root]
    while stack:
        expr_id = stack.pop()
        if expr_id in reachable:
            continue
        reachable.add(expr_id)
        stack.extend(expression_map[expr_id]["children"])
    unreachable = sorted(set(expression_map) - reachable)
    if unreachable:
        fail(f"{path}.body.expressions", f"contains unreachable expression id(s): {unreachable}")

    for index, raw in enumerate(array_value(function["contracts"], f"{path}.contracts")):
        contract_path = f"{path}.contracts[{index}]"
        contract = object_value(raw, contract_path)
        exact_keys(contract, contract_path, required={"kind", "expression_id"}, optional={"result_name"})
        kind = string_value(contract["kind"], f"{contract_path}.kind")
        if kind not in CONTRACT_KINDS:
            fail(f"{contract_path}.kind", f"unsupported contract kind: {kind}")
        expression_id = int_value(contract["expression_id"], f"{contract_path}.expression_id", minimum=0)
        if expression_id not in expression_map:
            fail(f"{contract_path}.expression_id", f"unknown expression id: {expression_id}")
        if kind == "ensures":
            string_value(contract.get("result_name"), f"{contract_path}.result_name")
        elif "result_name" in contract:
            fail(f"{contract_path}.result_name", "only valid for ensures")


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
    exact_keys(profile, "$.target_profile", required={"integer_model", "overflow", "floating_point", "pointer_width"})
    for field in ("integer_model", "overflow", "floating_point"):
        string_value(profile[field], f"$.target_profile.{field}")
    if int_value(profile["pointer_width"], "$.target_profile.pointer_width") not in {32, 64}:
        fail("$.target_profile.pointer_width", "expected 32 or 64")

    types = array_value(document["types"], "$.types")
    type_map = unique_id_map(types, "$.types")
    if 0 not in type_map or type_map[0].get("kind") != "unit" or type_map[0].get("value_type") != "void":
        fail("$.types", "type id 0 must be unit/void")
    type_wasm: dict[int, list[str]] = {}
    seen_type_keys: set[tuple[str, str]] = set()
    for index, ty in enumerate(types):
        _type(ty, f"$.types[{index}]")
        key = (ty["name"], ty["value_type"])
        if key in seen_type_keys:
            fail(f"$.types[{index}]", f"duplicate type identity: {key[0]} / {key[1]}")
        seen_type_keys.add(key)
        type_wasm[ty["id"]] = ty["representation"]["wasm"]

    functions = array_value(document["functions"], "$.functions")
    unique_id_map(functions, "$.functions")
    seen_names: set[str] = set()
    for index, function in enumerate(functions):
        path = f"$.functions[{index}]"
        _function(function, path, set(type_map), type_wasm)
        name = function["name"]
        if name in seen_names:
            fail(f"{path}.name", f"duplicate function name: {name}")
        seen_names.add(name)
    return document
