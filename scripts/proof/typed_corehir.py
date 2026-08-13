"""Independent public validator for TypedCoreHIR v1 artifacts."""

from __future__ import annotations

import copy
from typing import Any

from proof.common import object_value, validate_header
from proof import typed_corehir_impl as _typed_corehir_impl

SCHEMA = "arukellt-typed-corehir"
VERSION = 1


def _validate_explicit_type_metadata(document: dict[str, Any]) -> None:
    types = document.get("types")
    if not isinstance(types, list) or not types:
        raise ValueError("$.types: expected non-empty array")
    for index, entry in enumerate(types):
        path = f"$.types[{index}]"
        if not isinstance(entry, dict):
            raise ValueError(f"{path}: expected object")
        kind = entry.get("kind")
        if kind == "integer":
            if set(entry) != {
                "id", "kind", "name", "bits", "signed", "value_type", "representation"
            }:
                raise ValueError(
                    f"{path}: integer type requires explicit bits and signed fields"
                )
            bits = entry.get("bits")
            signed = entry.get("signed")
            if type(bits) is not int or bits not in {32, 64}:
                raise ValueError(f"{path}.bits: expected 32 or 64")
            if signed is not True:
                raise ValueError(f"{path}.signed: only signed proof integers are supported")
        elif "bits" in entry or "signed" in entry:
            raise ValueError(f"{path}: non-integer type must not carry bits or signed")


def _compatibility_root_set(document: dict[str, Any]) -> None:
    """Represent body and contract roots as one validation-only root."""

    for function in document.get("functions", []):
        contracts = function.get("contracts", [])
        if not contracts:
            continue
        body = function.get("body")
        if not isinstance(body, dict):
            continue
        expressions = body.get("expressions")
        root = body.get("root_expr_id")
        if not isinstance(expressions, list) or not isinstance(root, int):
            continue
        existing_ids = [
            expression.get("id")
            for expression in expressions
            if isinstance(expression, dict) and isinstance(expression.get("id"), int)
        ]
        synthetic_id = max(existing_ids, default=-1) + 1
        roots = [root]
        for contract in contracts:
            if not isinstance(contract, dict):
                continue
            expression_id = contract.get("expression_id")
            if isinstance(expression_id, int) and expression_id not in roots:
                roots.append(expression_id)
        expressions.append(
            {
                "id": synthetic_id,
                "kind": "proof-root-set",
                "kind_id": 20,
                "type_id": 0,
                "value_type": "void",
                "text": "",
                "int_value": 0,
                "float_value": 0.0,
                "span_start": 0,
                "children": roots,
            }
        )
        body["root_expr_id"] = synthetic_id


def validate_document(value: Any) -> dict[str, Any]:
    document = object_value(value, "$")
    validate_header(document, "$", SCHEMA, VERSION)
    _validate_explicit_type_metadata(document)

    compatibility = copy.deepcopy(document)
    compatibility["schema"] = _typed_corehir_impl.SCHEMA
    compatibility["schema_version"] = _typed_corehir_impl.VERSION
    for entry in compatibility.get("types", []):
        if isinstance(entry, dict):
            entry.pop("bits", None)
            entry.pop("signed", None)
    _compatibility_root_set(compatibility)
    _typed_corehir_impl.validate_document(compatibility)
    return document


__all__ = ["SCHEMA", "VERSION", "validate_document"]
