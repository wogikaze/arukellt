"""Fail-closed TypedCoreHIR v1 to VerifiedCore v1 conversion."""

from __future__ import annotations

import copy
from typing import Any

from proof.common import array_value, exact_keys, int_value, object_value, string_value, validate_header
from proof.verified_core import validate_document as validate_verified_core

SOURCE_SCHEMA = "arukellt-typed-corehir"
SOURCE_VERSION = 1


class UnsupportedTypedCoreHir(ValueError):
    """Raised when a valid-looking TypedCoreHIR artifact is outside the conversion subset."""


def _type_table(document: dict[str, Any]) -> tuple[list[dict[str, Any]], dict[int, dict[str, Any]]]:
    rendered: list[dict[str, Any]] = []
    by_id: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(array_value(document["types"], "$.types")):
        path = f"$.types[{index}]"
        entry = object_value(raw, path)
        exact_keys(entry, path, required={"id", "kind", "name", "value_type", "representation"})
        type_id = int_value(entry["id"], f"{path}.id", minimum=0)
        if type_id in by_id:
            raise UnsupportedTypedCoreHir(f"{path}.id: duplicate type id {type_id}")
        kind = string_value(entry["kind"], f"{path}.kind")
        name = string_value(entry["name"], f"{path}.name")
        representation = object_value(entry["representation"], f"{path}.representation")
        exact_keys(
            representation,
            f"{path}.representation",
            required={"kind", "wasm", "nullable", "size_bytes", "align_bytes"},
        )
        verified_type: dict[str, Any] = {
            "id": type_id,
            "kind": kind,
            "name": name,
            "representation": {
                "wasm": copy.deepcopy(representation["wasm"]),
                "nullable": representation["nullable"],
                "size_bytes": representation["size_bytes"],
                "align_bytes": representation["align_bytes"],
            },
        }
        if kind == "integer":
            if name == "i32":
                verified_type["bits"] = 32
                verified_type["signed"] = True
            elif name == "i64":
                verified_type["bits"] = 64
                verified_type["signed"] = True
            else:
                raise UnsupportedTypedCoreHir(f"{path}.name: unsupported integer type {name!r}")
        elif kind not in {"unit", "bool"}:
            raise UnsupportedTypedCoreHir(f"{path}.kind: unsupported proof type {kind!r}")
        rendered.append(verified_type)
        by_id[type_id] = entry
    if not rendered:
        raise UnsupportedTypedCoreHir("$.types: at least one type is required")
    return rendered, by_id


def _locals(function: dict[str, Any], path: str, known_types: set[int]) -> tuple[list[dict[str, Any]], dict[str, int]]:
    rendered: list[dict[str, Any]] = []
    name_to_id: dict[str, int] = {}
    for index, raw in enumerate(array_value(function["locals"], f"{path}.locals")):
        local_path = f"{path}.locals[{index}]"
        local = object_value(raw, local_path)
        exact_keys(local, local_path, required={"id", "name", "type_id", "storage"})
        local_id = int_value(local["id"], f"{local_path}.id", minimum=0)
        name = string_value(local["name"], f"{local_path}.name")
        type_id = int_value(local["type_id"], f"{local_path}.type_id", minimum=0)
        if type_id not in known_types:
            raise UnsupportedTypedCoreHir(f"{local_path}.type_id: unknown type {type_id}")
        if name in name_to_id:
            raise UnsupportedTypedCoreHir(f"{local_path}.name: duplicate local name {name!r}")
        name_to_id[name] = local_id
        rendered.append(
            {
                "id": local_id,
                "name": name,
                "type_id": type_id,
                "storage": string_value(local["storage"], f"{local_path}.storage"),
            }
        )
    return rendered, name_to_id


def _expression_index(function: dict[str, Any], path: str) -> tuple[int, dict[int, dict[str, Any]]]:
    body = object_value(function["body"], f"{path}.body")
    exact_keys(body, f"{path}.body", required={"root_expr_id", "expressions"})
    root = int_value(body["root_expr_id"], f"{path}.body.root_expr_id", minimum=0)
    by_id: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(array_value(body["expressions"], f"{path}.body.expressions")):
        expr_path = f"{path}.body.expressions[{index}]"
        expr = object_value(raw, expr_path)
        exact_keys(
            expr,
            expr_path,
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
        expr_id = int_value(expr["id"], f"{expr_path}.id", minimum=0)
        if expr_id in by_id:
            raise UnsupportedTypedCoreHir(f"{expr_path}.id: duplicate expression id {expr_id}")
        by_id[expr_id] = expr
    if root not in by_id:
        raise UnsupportedTypedCoreHir(f"{path}.body.root_expr_id: unknown expression {root}")
    for expr_id, expr in by_id.items():
        for child_index, child in enumerate(array_value(expr["children"], f"{path}.body.expressions[{expr_id}].children")):
            child_id = int_value(child, f"{path}.body.expressions[{expr_id}].children[{child_index}]", minimum=0)
            if child_id not in by_id:
                raise UnsupportedTypedCoreHir(
                    f"{path}.body.expressions[{expr_id}].children[{child_index}]: unknown expression {child_id}"
                )
    return root, by_id


_BINARY_KINDS = {
    "+": "add",
    "-": "sub",
    "*": "mul",
    "/": "div",
    "%": "mod",
    "==": "eq",
    "!=": "ne",
    "<": "lt",
    "<=": "le",
    ">": "gt",
    ">=": "ge",
    "&&": "and",
    "||": "or",
}
_UNARY_KINDS = {"!": "not", "-": "neg"}


def _proof_expression(
    expr_id: int,
    expressions: dict[int, dict[str, Any]],
    local_ids: dict[str, int],
    result_name: str,
    stack: set[int],
    next_id: list[int],
    path: str,
) -> dict[str, Any]:
    if expr_id in stack:
        raise UnsupportedTypedCoreHir(f"{path}: expression cycle at {expr_id}")
    stack = set(stack)
    stack.add(expr_id)
    expr = expressions[expr_id]
    kind = string_value(expr["kind"], f"{path}.kind")
    type_id = int_value(expr["type_id"], f"{path}.type_id", minimum=0)
    text = expr["text"]
    children = [int_value(value, f"{path}.children", minimum=0) for value in expr["children"]]
    verified_id = next_id[0]
    next_id[0] += 1
    common: dict[str, Any] = {"id": verified_id, "type_id": type_id}

    if kind in {"ident", "path"}:
        name = string_value(text, f"{path}.text")
        if result_name and name == result_name:
            return {**common, "kind": "result"}
        if name not in local_ids:
            raise UnsupportedTypedCoreHir(f"{path}.text: unknown proof identifier {name!r}")
        return {**common, "kind": "local", "local_id": local_ids[name]}
    if kind == "int":
        return {**common, "kind": "constant", "value": int_value(expr["int_value"], f"{path}.int_value")}
    if kind == "bool":
        normalized = str(text).lower()
        if normalized not in {"true", "false"}:
            raise UnsupportedTypedCoreHir(f"{path}.text: invalid boolean literal {text!r}")
        return {**common, "kind": "constant", "value": normalized == "true"}
    if kind == "binary":
        operator = string_value(text, f"{path}.text")
        verified_kind = _BINARY_KINDS.get(operator)
        if verified_kind is None:
            raise UnsupportedTypedCoreHir(f"{path}.text: unsupported binary operator {operator!r}")
        if len(children) != 2:
            raise UnsupportedTypedCoreHir(f"{path}.children: binary expression requires two children")
        return {
            **common,
            "kind": verified_kind,
            "operands": [
                _proof_expression(children[0], expressions, local_ids, result_name, stack, next_id, f"{path}.children[0]"),
                _proof_expression(children[1], expressions, local_ids, result_name, stack, next_id, f"{path}.children[1]"),
            ],
        }
    if kind == "unary":
        operator = string_value(text, f"{path}.text")
        verified_kind = _UNARY_KINDS.get(operator)
        if verified_kind is None:
            raise UnsupportedTypedCoreHir(f"{path}.text: unsupported unary operator {operator!r}")
        if len(children) != 1:
            raise UnsupportedTypedCoreHir(f"{path}.children: unary expression requires one child")
        return {
            **common,
            "kind": verified_kind,
            "operands": [
                _proof_expression(children[0], expressions, local_ids, result_name, stack, next_id, f"{path}.children[0]")
            ],
        }
    raise UnsupportedTypedCoreHir(f"{path}.kind: unsupported proof expression kind {kind!r}")


def _return_value(
    root_id: int,
    expressions: dict[int, dict[str, Any]],
    contract_ids: set[int],
    local_ids: dict[str, int],
    path: str,
) -> dict[str, Any]:
    root = expressions[root_id]
    candidates: list[int]
    if root["kind"] == "block":
        candidates = [int(child) for child in root["children"] if int(child) not in contract_ids]
    else:
        candidates = [root_id] if root_id not in contract_ids else []
    if len(candidates) != 1:
        raise UnsupportedTypedCoreHir(
            f"{path}.body: expected exactly one non-contract return expression, found {len(candidates)}"
        )
    expr = expressions[candidates[0]]
    type_id = int_value(expr["type_id"], f"{path}.body.return.type_id", minimum=0)
    if expr["kind"] in {"ident", "path"}:
        name = string_value(expr["text"], f"{path}.body.return.text")
        if name not in local_ids:
            raise UnsupportedTypedCoreHir(f"{path}.body.return.text: unknown local {name!r}")
        return {"kind": "local", "type_id": type_id, "local_id": local_ids[name]}
    if expr["kind"] == "int":
        return {"kind": "constant", "type_id": type_id, "value": int(expr["int_value"])}
    if expr["kind"] == "bool":
        normalized = str(expr["text"]).lower()
        if normalized not in {"true", "false"}:
            raise UnsupportedTypedCoreHir(f"{path}.body.return.text: invalid boolean literal")
        return {"kind": "constant", "type_id": type_id, "value": normalized == "true"}
    raise UnsupportedTypedCoreHir(
        f"{path}.body.return.kind: unsupported executable return expression {expr['kind']!r}"
    )


def convert_document(value: Any) -> dict[str, Any]:
    source = object_value(value, "$")
    validate_header(source, "$", SOURCE_SCHEMA, SOURCE_VERSION)
    exact_keys(
        source,
        "$",
        required={"schema", "schema_version", "generator", "module", "target_profile", "types", "functions"},
    )
    verified_types, source_types = _type_table(source)
    known_types = set(source_types)
    verified_functions: list[dict[str, Any]] = []

    for function_index, raw in enumerate(array_value(source["functions"], "$.functions")):
        path = f"$.functions[{function_index}]"
        function = object_value(raw, path)
        exact_keys(
            function,
            path,
            required={"id", "name", "signature", "abi", "locals", "contracts", "body"},
        )
        rendered_locals, local_ids = _locals(function, path, known_types)
        root_id, expressions = _expression_index(function, path)
        contracts: list[dict[str, Any]] = []
        contract_ids: set[int] = set()
        next_proof_id = [0]
        for contract_index, raw_contract in enumerate(array_value(function["contracts"], f"{path}.contracts")):
            contract_path = f"{path}.contracts[{contract_index}]"
            contract = object_value(raw_contract, contract_path)
            exact_keys(contract, contract_path, required={"kind", "expression_id"}, optional={"result_name"})
            kind = string_value(contract["kind"], f"{contract_path}.kind")
            if kind not in {"requires", "ensures"}:
                raise UnsupportedTypedCoreHir(f"{contract_path}.kind: unsupported contract {kind!r}")
            expression_id = int_value(contract["expression_id"], f"{contract_path}.expression_id", minimum=0)
            if expression_id not in expressions:
                raise UnsupportedTypedCoreHir(f"{contract_path}.expression_id: unknown expression {expression_id}")
            contract_ids.add(expression_id)
            result_name = str(contract.get("result_name", "result" if kind == "ensures" else ""))
            rendered: dict[str, Any] = {
                "kind": kind,
                "expression": _proof_expression(
                    expression_id,
                    expressions,
                    local_ids,
                    result_name,
                    set(),
                    next_proof_id,
                    f"{contract_path}.expression",
                ),
            }
            if result_name:
                rendered["result_name"] = result_name
            contracts.append(rendered)
        if not contracts:
            raise UnsupportedTypedCoreHir(f"{path}.contracts: at least one contract is required")

        return_value = _return_value(root_id, expressions, contract_ids, local_ids, path)
        verified_functions.append(
            {
                "id": function["id"],
                "name": function["name"],
                "signature": copy.deepcopy(function["signature"]),
                "abi": copy.deepcopy(function["abi"]),
                "locals": rendered_locals,
                "contracts": contracts,
                "body": {
                    "entry_block": 0,
                    "blocks": [
                        {
                            "id": 0,
                            "parameters": [],
                            "instructions": [],
                            "terminator": {"kind": "return", "value": return_value},
                        }
                    ],
                },
            }
        )

    result = {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": "arukellt-typed-corehir-converter-v1",
        "module": source["module"],
        "target_profile": copy.deepcopy(source["target_profile"]),
        "types": verified_types,
        "functions": verified_functions,
    }
    return validate_verified_core(result)


__all__ = ["SOURCE_SCHEMA", "SOURCE_VERSION", "UnsupportedTypedCoreHir", "convert_document"]
