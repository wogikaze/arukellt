"""Proof-facing TypedCoreHIR admission with Phase 5 aggregate metadata."""
from __future__ import annotations
import copy
from typing import Any
from proof import typed_corehir as v1
from proof.aggregate_types import validate_aggregate_types

SCHEMA = "arukellt-typed-corehir"
VERSION = 2
AGGREGATE_KINDS = {"construct", "project", "is_variant", "variant_payload"}


def validate_document(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict): raise ValueError("$: expected object")
    if value.get("schema") != SCHEMA: raise ValueError("$.schema: unsupported TypedCoreHIR schema")
    if value.get("schema_version") == 1: return v1.validate_document(value)
    if value.get("schema_version") != VERSION: raise ValueError("$.schema_version: expected 1 or 2")
    document = copy.deepcopy(value)
    for key in ("module", "target_profile", "types", "functions"):
        if key not in document: raise ValueError(f"$.{key}: missing")
    if not isinstance(document["types"], list) or not document["types"]: raise ValueError("$.types: expected non-empty array")
    type_ids = {int(entry["id"]) for entry in document["types"]}
    if len(type_ids) != len(document["types"]): raise ValueError("$.types: duplicate TypeId")
    validate_aggregate_types(document)
    function_ids: set[int] = set()
    for function_index, function in enumerate(document["functions"]):
        path = f"$.functions[{function_index}]"; function_id = int(function["id"])
        if function_id in function_ids: raise ValueError(f"{path}.id: duplicate function id")
        function_ids.add(function_id)
        signature = function["signature"]
        return_type = int(signature["return_type_id"])
        if return_type not in type_ids: raise ValueError(f"{path}.signature.return_type_id: unknown TypeId")
        for parameter_index, parameter in enumerate(signature["parameters"]):
            if int(parameter["type_id"]) not in type_ids: raise ValueError(f"{path}.signature.parameters[{parameter_index}].type_id: unknown TypeId")
        local_ids: set[int] = set()
        for local_index, local in enumerate(function["locals"]):
            local_id = int(local["id"])
            if local_id in local_ids: raise ValueError(f"{path}.locals[{local_index}].id: duplicate local id")
            local_ids.add(local_id)
            if int(local["type_id"]) not in type_ids: raise ValueError(f"{path}.locals[{local_index}].type_id: unknown TypeId")
        body = function["body"]; expressions = body["expressions"]
        if not isinstance(expressions, list) or not expressions: raise ValueError(f"{path}.body.expressions: expected non-empty array")
        expression_ids = {int(expression["id"]) for expression in expressions}
        if len(expression_ids) != len(expressions): raise ValueError(f"{path}.body.expressions: duplicate expression id")
        if int(body["root_expression_id"]) not in expression_ids: raise ValueError(f"{path}.body.root_expression_id: unknown expression")
        for expression_index, expression in enumerate(expressions):
            expression_path = f"{path}.body.expressions[{expression_index}]"
            if int(expression["type_id"]) not in type_ids: raise ValueError(f"{expression_path}.type_id: unknown TypeId")
            children = expression.get("children", [])
            if not isinstance(children, list) or any(int(child) not in expression_ids for child in children): raise ValueError(f"{expression_path}.children: unknown expression id")
            kind = str(expression["kind"])
            if kind in AGGREGATE_KINDS:
                if kind in {"project", "is_variant", "variant_payload"} and len(children) != 1: raise ValueError(f"{expression_path}.children: aggregate selector requires one source")
                if kind == "project" and type(expression.get("index")) is not int: raise ValueError(f"{expression_path}.index: required")
                if kind in {"is_variant", "variant_payload"} and type(expression.get("variant_index")) is not int: raise ValueError(f"{expression_path}.variant_index: required")
                if kind == "variant_payload" and type(expression.get("payload_index")) is not int: raise ValueError(f"{expression_path}.payload_index: required")
        for contract_index, contract in enumerate(function["contracts"]):
            if int(contract["expression_id"]) not in expression_ids: raise ValueError(f"{path}.contracts[{contract_index}].expression_id: unknown expression")
    return document


__all__ = ["AGGREGATE_KINDS", "SCHEMA", "VERSION", "validate_document"]
