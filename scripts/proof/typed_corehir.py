"""Independent public validator for typed CoreHIR v1 artifacts."""

from __future__ import annotations

import copy
from typing import Any

from proof.common import object_value, validate_header
from proof import typed_corehir_impl as _typed_corehir_impl

SCHEMA = "arukellt-typed-corehir"
VERSION = 1


def _compatibility_root_set(document: dict[str, Any]) -> None:
    """Represent the body root and contract roots as one validation-only root.

    ``typed_corehir_impl`` predates structured contracts and therefore knows a
    single expression root. TypedCoreHIR functions have a body root plus zero
    or more contract roots. A synthetic unit block in the deep-copied document
    lets the existing independent validator check the union without weakening
    its unreachable-expression rejection.
    """

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
    compatibility = copy.deepcopy(document)
    compatibility["schema"] = _typed_corehir_impl.SCHEMA
    compatibility["schema_version"] = _typed_corehir_impl.VERSION
    _compatibility_root_set(compatibility)
    _typed_corehir_impl.validate_document(compatibility)
    return document


__all__ = ["SCHEMA", "VERSION", "validate_document"]
