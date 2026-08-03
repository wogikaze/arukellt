"""Fail-closed TypedCoreHIR v1 to VerifiedCore v1 conversion.

Only functions carrying contracts are promoted to VerifiedCore. Uncontracted
runtime helpers and unrelated type-table entries remain outside the proof
subject rather than becoming implicit obligations or trusted assumptions.
"""

from __future__ import annotations

import copy
from typing import Any

from proof.common import array_value, int_value, object_value, string_value
from proof.typed_corehir import validate_document as validate_typed_corehir
from proof.verified_core import validate_document as validate_verified_core


class UnsupportedTypedCoreHir(ValueError):
    """Valid TypedCoreHIR that is outside the current proof conversion subset."""


def _index_types(document: dict[str, Any]) -> dict[int, dict[str, Any]]:
    by_id: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(array_value(document["types"], "$.types")):
        entry = object_value(raw, f"$.types[{index}]")
        type_id = int_value(entry["id"], f"$.types[{index}].id", minimum=0)
        if type_id in by_id:
            raise UnsupportedTypedCoreHir(f"$.types[{index}].id: duplicate type id {type_id}")
        by_id[type_id] = entry
    return by_id


def _verified_type(entry: dict[str, Any], path: str) -> dict[str, Any]:
    kind = string_value(entry["kind"], f"{path}.kind")
    name = string_value(entry["name"], f"{path}.name")
    representation = object_value(entry["representation"], f"{path}.representation")
    rendered: dict[str, Any] = {
        "id": int_value(entry["id"], f"{path}.id", minimum=0),
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
            rendered.update(bits=32, signed=True)
        elif name == "i64":
            rendered.update(bits=64, signed=True)
        else:
            raise UnsupportedTypedCoreHir(f"{path}.name: unsupported integer type {name!r}")
    elif kind not in {"unit", "bool"}:
        raise UnsupportedTypedCoreHir(f"{path}.kind: unsupported reachable proof type {kind!r}")
    return rendered


def _locals(
    function: dict[str, Any], path: str
) -> tuple[list[dict[str, Any]], dict[str, int], set[int]]:
    rendered: list[dict[str, Any]] = []
    name_to_id: dict[str, int] = {}
    used_types: set[int] = set()
    for index, raw in enumerate(array_value(function["locals"], f"{path}.locals")):
        local_path = f"{path}.locals[{index}]"
        local = object_value(raw, local_path)
        local_id = int_value(local["id"], f"{local_path}.id", minimum=0)
        name = string_value(local["name"], f"{local_path}.name")
        type_id = int_value(local["type_id"], f"{local_path}.type_id", minimum=0)
        if name in name_to_id:
            raise UnsupportedTypedCoreHir(f"{local_path}.name: duplicate local name {name!r}")
        name_to_id[name] = local_id
        used_types.add(type_id)
        rendered.append(
            {
                "id": local_id,
                "name": name,
                "type_id": type_id,
                "storage": string_value(local["storage"], f"{local_path}.storage"),
            }
        )
    return rendered, name_to_id, used_types


def _expression_index(
    function: dict[str, Any], path: str
) -> tuple[int, dict[int, dict[str, Any]]]:
    body = object_value(function["body"], f"{path}.body")
    root = int_value(body["root_expr_id"], f"{path}.body.root_expr_id", minimum=0)
    by_id: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(array_value(body["expressions"], f"{path}.body.expressions")):
        expr_path = f"{path}.body.expressions[{index}]"
        expr = object_value(raw, expr_path)
        expr_id = int_value(expr["id"], f"{expr_path}.id", minimum=0)
        if expr_id in by_id:
            raise UnsupportedTypedCoreHir(f"{expr_path}.id: duplicate expression id {expr_id}")
        by_id[expr_id] = expr
    if root not in by_id:
        raise UnsupportedTypedCoreHir(f"{path}.body.root_expr_id: unknown expression {root}")
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
    source_id: int,
    expressions: dict[int, dict[str, Any]],
    local_ids: dict[str, int],
    result_name: str,
    stack: set[int],
    next_id: list[int],
    used_types: set[int],
    path: str,
) -> dict[str, Any]:
    if source_id in stack:
        raise UnsupportedTypedCoreHir(f"{path}: expression cycle at {source_id}")
    if source_id not in expressions:
        raise UnsupportedTypedCoreHir(f"{path}: unknown expression {source_id}")
    stack = {*stack, source_id}
    expr = expressions[source_id]
    kind = string_value(expr["kind"], f"{path}.kind")
    type_id = int_value(expr["type_id"], f"{path}.type_id", minimum=0)
    used_types.add(type_id)
    text = str(expr["text"])
    children = [int_value(value, f"{path}.children", minimum=0) for value in expr["children"]]
    verified_id = next_id[0]
    next_id[0] += 1
    common: dict[str, Any] = {"id": verified_id, "type_id": type_id}

    if kind in {"ident", "path"}:
        if result_name and text == result_name:
            return {**common, "kind": "result"}
        if text not in local_ids:
            raise UnsupportedTypedCoreHir(f"{path}.text: unknown proof identifier {text!r}")
        return {**common, "kind": "local", "local_id": local_ids[text]}
    if kind == "int":
        return {**common, "kind": "constant", "value": int(expr["int_value"])}
    if kind == "bool":
        normalized = text.lower()
        if normalized not in {"true", "false"}:
            raise UnsupportedTypedCoreHir(f"{path}.text: invalid boolean literal {text!r}")
        return {**common, "kind": "constant", "value": normalized == "true"}
    if kind == "binary":
        verified_kind = _BINARY_KINDS.get(text)
        if verified_kind is None or len(children) != 2:
            raise UnsupportedTypedCoreHir(f"{path}: unsupported binary expression {text!r}")
        return {
            **common,
            "kind": verified_kind,
            "operands": [
                _proof_expression(
                    children[0], expressions, local_ids, result_name, stack,
                    next_id, used_types, f"{path}.children[0]"
                ),
                _proof_expression(
                    children[1], expressions, local_ids, result_name, stack,
                    next_id, used_types, f"{path}.children[1]"
                ),
            ],
        }
    if kind == "unary":
        verified_kind = _UNARY_KINDS.get(text)
        if verified_kind is None or len(children) != 1:
            raise UnsupportedTypedCoreHir(f"{path}: unsupported unary expression {text!r}")
        return {
            **common,
            "kind": verified_kind,
            "operands": [
                _proof_expression(
                    children[0], expressions, local_ids, result_name, stack,
                    next_id, used_types, f"{path}.children[0]"
                )
            ],
        }
    raise UnsupportedTypedCoreHir(f"{path}.kind: unsupported proof expression kind {kind!r}")


def _return_value(
    root_id: int,
    expressions: dict[int, dict[str, Any]],
    contract_ids: set[int],
    local_ids: dict[str, int],
    used_types: set[int],
    path: str,
) -> dict[str, Any]:
    root = expressions[root_id]
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
    used_types.add(type_id)
    kind = str(expr["kind"])
    if kind in {"ident", "path"}:
        name = str(expr["text"])
        if name not in local_ids:
            raise UnsupportedTypedCoreHir(f"{path}.body.return.text: unknown local {name!r}")
        return {"kind": "local", "type_id": type_id, "local_id": local_ids[name]}
    if kind == "int":
        return {"kind": "constant", "type_id": type_id, "value": int(expr["int_value"])}
    if kind == "bool":
        normalized = str(expr["text"]).lower()
        if normalized not in {"true", "false"}:
            raise UnsupportedTypedCoreHir(f"{path}.body.return.text: invalid boolean literal")
        return {"kind": "constant", "type_id": type_id, "value": normalized == "true"}
    raise UnsupportedTypedCoreHir(
        f"{path}.body.return.kind: unsupported executable return expression {kind!r}"
    )


def convert_document(value: Any) -> dict[str, Any]:
    source = validate_typed_corehir(value)
    source_types = _index_types(source)
    if 0 not in source_types or source_types[0].get("kind") != "unit":
        raise UnsupportedTypedCoreHir("$.types: type id 0 must be unit")
    # VerifiedCore v1 reserves type id 0 for unit even when the contracted
    # function does not directly mention unit.
    used_types: set[int] = {0}
    verified_functions: list[dict[str, Any]] = []

    for function_index, raw in enumerate(array_value(source["functions"], "$.functions")):
        path = f"$.functions[{function_index}]"
        function = object_value(raw, path)
        source_contracts = array_value(function["contracts"], f"{path}.contracts")
        if not source_contracts:
            continue

        rendered_locals, local_ids, local_types = _locals(function, path)
        used_types.update(local_types)
        signature = object_value(function["signature"], f"{path}.signature")
        used_types.add(int(signature["return_type_id"]))
        for parameter in signature["parameters"]:
            used_types.add(int(parameter["type_id"]))

        root_id, expressions = _expression_index(function, path)
        contracts: list[dict[str, Any]] = []
        contract_ids: set[int] = set()
        next_proof_id = [0]
        for contract_index, raw_contract in enumerate(source_contracts):
            contract_path = f"{path}.contracts[{contract_index}]"
            contract = object_value(raw_contract, contract_path)
            kind = string_value(contract["kind"], f"{contract_path}.kind")
            if kind not in {"requires", "ensures"}:
                raise UnsupportedTypedCoreHir(f"{contract_path}.kind: unsupported contract {kind!r}")
            expression_id = int_value(
                contract["expression_id"], f"{contract_path}.expression_id", minimum=0
            )
            contract_ids.add(expression_id)
            result_name = str(contract.get("result_name", "result" if kind == "ensures" else ""))
            rendered: dict[str, Any] = {
                "kind": kind,
                "expression": _proof_expression(
                    expression_id, expressions, local_ids, result_name, set(),
                    next_proof_id, used_types, f"{contract_path}.expression"
                ),
            }
            if result_name:
                rendered["result_name"] = result_name
            contracts.append(rendered)

        return_value = _return_value(
            root_id, expressions, contract_ids, local_ids, used_types, path
        )
        verified_functions.append(
            {
                "id": function["id"],
                "name": function["name"],
                "signature": copy.deepcopy(signature),
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

    if not verified_functions:
        raise UnsupportedTypedCoreHir("$.functions: no contracted functions")
    missing = sorted(type_id for type_id in used_types if type_id not in source_types)
    if missing:
        raise UnsupportedTypedCoreHir(f"$.types: missing reachable type ids {missing}")
    verified_types = [
        _verified_type(source_types[type_id], f"$.types[id={type_id}]")
        for type_id in sorted(used_types)
    ]

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


__all__ = ["UnsupportedTypedCoreHir", "convert_document"]
