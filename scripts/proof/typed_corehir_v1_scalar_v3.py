"""Fail-closed scalar TypedCoreHIR v1 -> canonical v3 proof-source bridge.

This bridge is intentionally narrower than either schema.  It upgrades only the
selfhost subset whose machine/checked/no-float profile is observationally safe:
unit/bool/signed i32/i64 values, constants, locals/results, blocks, comparisons,
and boolean logic.  Overflow-capable arithmetic, references, floating point,
calls, control flow, and unknown expression kinds are rejected before a v3
artifact can be produced.
"""
from __future__ import annotations

import copy
from typing import Any

from proof.typed_corehir import validate_document as validate_v1_document
from proof.typed_corehir_v3 import validate_document as validate_v3_document

GENERATOR = "arukellt-selfhost-v1-scalar-upgrade-v1"
CAPABILITY_PROFILE = "proof-phases-0-7"

_BINARY = {
    "==": "eq",
    "!=": "ne",
    "<": "lt",
    "<=": "le",
    ">": "gt",
    ">=": "ge",
    "&&": "and",
    "||": "or",
}
_UNARY = {"!": "not"}
_ALLOWED_TYPE_KINDS = {"unit", "bool", "integer"}
_SELFHOST_NON_INTEGER_TYPE_IDS = {3, 5, 6, 7, 8, 9}


def _fail(path: str, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def _children(expression: dict[str, Any], path: str) -> list[int]:
    raw = expression.get("children", [])
    if not isinstance(raw, list):
        _fail(f"{path}.children", "expected array")
    return [int(value) for value in raw]


def _reachable(
    root: int,
    expressions: dict[int, dict[str, Any]],
    *,
    contract_roots: set[int],
    body: bool,
) -> set[int]:
    result: set[int] = set()
    stack = [root]
    while stack:
        expression_id = stack.pop()
        if expression_id in result:
            continue
        expression = expressions.get(expression_id)
        if expression is None:
            _fail("$.functions[].body", f"unknown expression id {expression_id}")
        result.add(expression_id)
        children = _children(expression, f"$.expressions[id={expression_id}]")
        if body and expression.get("kind") == "block":
            children = [child for child in children if child not in contract_roots]
        stack.extend(children)
    return result


def _type_entry(
    source: dict[str, Any],
    *,
    source_generator: str,
    path: str,
) -> dict[str, Any]:
    kind = str(source.get("kind", ""))
    if kind not in _ALLOWED_TYPE_KINDS:
        _fail(f"{path}.kind", f"scalar bridge rejects {kind!r}")
    type_id = int(source.get("id", -1))
    if source_generator == "arukellt-selfhost" and type_id in _SELFHOST_NON_INTEGER_TYPE_IDS:
        _fail(path, f"selfhost TypeId {type_id} is outside scalar i32/i64 proof semantics")
    representation = source.get("representation")
    if not isinstance(representation, dict):
        _fail(f"{path}.representation", "expected object")
    rendered_representation = {
        key: copy.deepcopy(value)
        for key, value in representation.items()
        if key in {"kind", "wasm", "nullable", "size_bytes", "align_bytes", "layout_id"}
    }
    result: dict[str, Any] = {
        "id": type_id,
        "kind": kind,
        "name": str(source.get("name", f"type-{type_id}")),
        "representation": rendered_representation,
    }
    if kind == "integer":
        bits = source.get("bits")
        signed = source.get("signed")
        if type(bits) is not int or bits not in {32, 64} or signed is not True:
            _fail(path, "integer requires explicit signed 32/64-bit metadata")
        expected = [f"i{bits}"]
        if representation.get("wasm") != expected or representation.get("nullable") is not False:
            _fail(f"{path}.representation", f"integer representation must be {expected!r} non-null")
        result["bits"] = bits
        result["signed"] = True
    elif kind == "bool":
        if representation.get("wasm") != ["i32"] or representation.get("nullable") is not False:
            _fail(f"{path}.representation", "bool must be non-null i32")
    else:
        if representation.get("wasm") != [] or representation.get("nullable") is not False:
            _fail(f"{path}.representation", "unit must have empty non-null representation")
    return result


def _canonical_expression(
    source: dict[str, Any],
    *,
    local_names: dict[str, int],
    result_names: set[str],
    contract_roots: set[int],
    body: bool,
    path: str,
) -> dict[str, Any]:
    expression_id = int(source["id"])
    type_id = int(source["type_id"])
    kind = str(source["kind"])
    children = _children(source, path)
    if body and kind == "block":
        children = [child for child in children if child not in contract_roots]
    result: dict[str, Any] = {"id": expression_id, "kind": kind, "type_id": type_id, "children": children}

    if kind in {"ident", "path"}:
        text = str(source.get("text", ""))
        is_local = text in local_names
        is_result = text in result_names
        if is_local and is_result:
            _fail(f"{path}.text", f"ambiguous local/result name {text!r}")
        if is_local:
            result["kind"] = "local"
            result["local_id"] = local_names[text]
            result["children"] = []
            return result
        if is_result:
            result["kind"] = "result"
            result["children"] = []
            return result
        _fail(f"{path}.text", f"unknown scalar proof identifier {text!r}")
    if kind == "int":
        result["kind"] = "constant"
        result["value"] = int(source.get("int_value", 0))
        result["children"] = []
        return result
    if kind == "bool":
        text = str(source.get("text", "")).lower()
        if text not in {"true", "false"}:
            _fail(f"{path}.text", "invalid bool literal")
        result["kind"] = "constant"
        result["value"] = text == "true"
        result["children"] = []
        return result
    if kind == "binary":
        operator = _BINARY.get(str(source.get("text", "")))
        if operator is None:
            _fail(path, "overflow-capable or unsupported binary operation is outside scalar bridge")
        if len(children) != 2:
            _fail(f"{path}.children", "binary operation requires two operands")
        result["kind"] = operator
        return result
    if kind == "unary":
        operator = _UNARY.get(str(source.get("text", "")))
        if operator is None:
            _fail(path, "overflow-capable or unsupported unary operation is outside scalar bridge")
        if len(children) != 1:
            _fail(f"{path}.children", "unary operation requires one operand")
        result["kind"] = operator
        return result
    if kind == "block":
        return result
    _fail(path, f"expression kind {kind!r} is outside scalar v1->v3 bridge")


def upgrade_scalar_document(value: Any) -> dict[str, Any]:
    source = validate_v1_document(value)
    source_generator = str(source.get("generator", ""))
    pointer_width = source.get("target_profile", {}).get("pointer_width")
    if pointer_width not in {32, 64}:
        _fail("$.target_profile.pointer_width", "expected 32 or 64")

    source_types = {int(entry["id"]): entry for entry in source["types"]}
    functions: list[dict[str, Any]] = []
    used_type_ids: set[int] = set()

    for function_index, function in enumerate(source["functions"]):
        contracts = function.get("contracts", [])
        if not contracts:
            continue
        path = f"$.functions[{function_index}]"
        locals_source = function.get("locals", [])
        local_names = {str(local["name"]): int(local["id"]) for local in locals_source}
        if len(local_names) != len(locals_source):
            _fail(f"{path}.locals", "duplicate local name")
        result_names = {
            str(contract.get("result_name", "result"))
            for contract in contracts
            if contract.get("kind") == "ensures"
        }
        contract_roots = {int(contract["expression_id"]) for contract in contracts}
        body = function["body"]
        root = int(body["root_expr_id"])
        expression_index = {int(expression["id"]): expression for expression in body["expressions"]}
        if len(expression_index) != len(body["expressions"]):
            _fail(f"{path}.body.expressions", "duplicate expression id")

        body_ids = _reachable(root, expression_index, contract_roots=contract_roots, body=True)
        proof_ids: set[int] = set()
        for contract_root in contract_roots:
            proof_ids |= _reachable(contract_root, expression_index, contract_roots=contract_roots, body=False)
        retained = body_ids | proof_ids
        canonical_expressions: list[dict[str, Any]] = []
        for expression_id in sorted(retained):
            expression = expression_index[expression_id]
            canonical = _canonical_expression(
                expression,
                local_names=local_names,
                result_names=result_names,
                contract_roots=contract_roots,
                body=expression_id in body_ids,
                path=f"{path}.body.expressions[id={expression_id}]",
            )
            used_type_ids.add(int(canonical["type_id"]))
            canonical_expressions.append(canonical)

        signature = copy.deepcopy(function["signature"])
        used_type_ids.add(int(signature["return_type_id"]))
        for parameter in signature["parameters"]:
            used_type_ids.add(int(parameter["type_id"]))
        locals_rendered = copy.deepcopy(locals_source)
        for local in locals_rendered:
            used_type_ids.add(int(local["type_id"]))
        abi = copy.deepcopy(function["abi"])
        for item in list(abi["parameters"]) + list(abi["results"]):
            used_type_ids.add(int(item["type_id"]))

        functions.append({
            "id": int(function["id"]),
            "name": str(function["name"]),
            "signature": signature,
            "abi": abi,
            "locals": locals_rendered,
            "contracts": copy.deepcopy(contracts),
            "body": {"root_expression_id": root, "expressions": canonical_expressions},
        })

    if not functions:
        _fail("$.functions", "no contracted functions")
    for required in (0,):
        if required in source_types:
            used_type_ids.add(required)
    bool_ids = [type_id for type_id, entry in source_types.items() if entry.get("kind") == "bool"]
    if len(bool_ids) != 1:
        _fail("$.types", "scalar bridge requires exactly one bool TypeId")
    used_type_ids.add(bool_ids[0])

    missing = sorted(type_id for type_id in used_type_ids if type_id not in source_types)
    if missing:
        _fail("$.types", f"missing used TypeIds {missing}")
    types = [
        _type_entry(
            source_types[type_id],
            source_generator=source_generator,
            path=f"$.types[id={type_id}]",
        )
        for type_id in sorted(used_type_ids)
    ]
    document = {
        "schema": "arukellt-typed-corehir",
        "schema_version": 3,
        "generator": GENERATOR,
        "capability_profile": CAPABILITY_PROFILE,
        "module": str(source["module"]),
        "target_profile": {
            "integer_model": "machine",
            "overflow": "checked",
            "floating_point": "unsupported",
            "pointer_width": pointer_width,
        },
        "types": types,
        "proof_memory": {"model": "arukellt-readonly-heap-v1", "references": []},
        "functions": functions,
    }
    return validate_v3_document(document)


__all__ = ["CAPABILITY_PROFILE", "GENERATOR", "upgrade_scalar_document"]
