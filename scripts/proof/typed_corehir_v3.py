"""Proof-facing TypedCoreHIR v3 admission for machine integers and read-only memory."""
from __future__ import annotations

import copy
from typing import Any

from proof import typed_corehir_v2 as v2
from proof.capabilities_v3 import PROFILE as CAPABILITY_PROFILE
from proof.machine_integer_semantics import SHIFT_OPERATORS, CONVERSIONS
from proof.readonly_memory_semantics import MEMORY_EXPRESSION_KINDS, MODEL

SCHEMA = "arukellt-typed-corehir"
VERSION = 3
CANONICAL_V3_KINDS = set(v2.AGGREGATE_KINDS) | set(MEMORY_EXPRESSION_KINDS) | set(SHIFT_OPERATORS) | {"convert"}


def _fail(path: str, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def validate_document(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        _fail("$", "expected object")
    if value.get("schema") != SCHEMA:
        _fail("$.schema", "unsupported TypedCoreHIR schema")
    version = value.get("schema_version")
    if version in {1, 2}:
        return v2.validate_document(value)
    if version != VERSION:
        _fail("$.schema_version", "expected 1, 2, or 3")
    document = copy.deepcopy(value)
    required = {"schema", "schema_version", "module", "target_profile", "types", "functions", "proof_memory"}
    optional = {"generator", "capability_profile"}
    if not required <= set(document) or set(document) - required - optional:
        _fail("$", "invalid TypedCoreHIR v3 top-level fields")
    if document.get("capability_profile", CAPABILITY_PROFILE) != CAPABILITY_PROFILE:
        _fail("$.capability_profile", f"expected {CAPABILITY_PROFILE!r}")
    profile = document["target_profile"]
    if not isinstance(profile, dict) or profile.get("integer_model") != "machine" or profile.get("overflow") != "checked" or profile.get("floating_point") != "unsupported":
        _fail("$.target_profile", "v3 requires machine/checked/no-float proof profile")
    types = document["types"]
    if not isinstance(types, list) or not types:
        _fail("$.types", "expected non-empty array")
    by_type: dict[int, dict[str, Any]] = {}
    for index, entry in enumerate(types):
        if not isinstance(entry, dict):
            _fail(f"$.types[{index}]", "expected object")
        type_id = int(entry.get("id", -1))
        if type_id < 0 or type_id in by_type:
            _fail(f"$.types[{index}].id", "invalid/duplicate TypeId")
        by_type[type_id] = entry
        if entry.get("kind") == "integer":
            if entry.get("bits") not in {32, 64} or entry.get("signed") is not True:
                _fail(f"$.types[{index}]", "machine integer requires explicit signed 32/64-bit metadata")
        if entry.get("kind") == "reference" and int(entry.get("pointee_type_id", -1)) < 0:
            _fail(f"$.types[{index}].pointee_type_id", "reference requires explicit pointee TypeId")
    type_ids = set(by_type)
    for type_id, entry in by_type.items():
        if entry.get("kind") == "reference" and int(entry["pointee_type_id"]) not in type_ids:
            _fail(f"$.types[id={type_id}].pointee_type_id", "unknown TypeId")
        for member in entry.get("elements", []):
            if int(member) not in type_ids:
                _fail(f"$.types[id={type_id}].elements", "unknown TypeId")
        for field in entry.get("fields", []):
            if int(field.get("type_id", -1)) not in type_ids:
                _fail(f"$.types[id={type_id}].fields", "unknown TypeId")
        for variant in entry.get("variants", []):
            for member in variant.get("payload_type_ids", []):
                if int(member) not in type_ids:
                    _fail(f"$.types[id={type_id}].variants", "unknown payload TypeId")
    memory = document["proof_memory"]
    if not isinstance(memory, dict) or memory.get("model") != MODEL or not isinstance(memory.get("references"), list):
        _fail("$.proof_memory", f"expected {MODEL!r} descriptor table")
    described = {int(entry.get("type_id", -1)) for entry in memory["references"]}
    reference_ids = {type_id for type_id, entry in by_type.items() if entry.get("kind") == "reference"}
    if described != reference_ids:
        _fail("$.proof_memory.references", "must describe exactly all reference TypeIds")

    function_ids: set[int] = set()
    for function_index, function in enumerate(document["functions"]):
        path = f"$.functions[{function_index}]"
        function_id = int(function.get("id", -1))
        if function_id < 0 or function_id in function_ids:
            _fail(f"{path}.id", "invalid/duplicate function id")
        function_ids.add(function_id)
        signature = function["signature"]
        if int(signature["return_type_id"]) not in type_ids:
            _fail(f"{path}.signature.return_type_id", "unknown TypeId")
        for parameter_index, parameter in enumerate(signature["parameters"]):
            if int(parameter["type_id"]) not in type_ids:
                _fail(f"{path}.signature.parameters[{parameter_index}].type_id", "unknown TypeId")
        local_ids: set[int] = set()
        for local_index, local in enumerate(function["locals"]):
            local_id = int(local["id"])
            if local_id in local_ids or int(local["type_id"]) not in type_ids:
                _fail(f"{path}.locals[{local_index}]", "duplicate local or unknown TypeId")
            local_ids.add(local_id)
        body = function["body"]
        expressions = body["expressions"]
        if not isinstance(expressions, list) or not expressions:
            _fail(f"{path}.body.expressions", "expected non-empty array")
        by_expression = {int(expression["id"]): expression for expression in expressions}
        if len(by_expression) != len(expressions) or int(body["root_expression_id"]) not in by_expression:
            _fail(f"{path}.body", "duplicate expression id or unknown root")
        for expression_index, expression in enumerate(expressions):
            expression_path = f"{path}.body.expressions[{expression_index}]"
            if int(expression["type_id"]) not in type_ids:
                _fail(f"{expression_path}.type_id", "unknown TypeId")
            children = expression.get("children", [])
            if not isinstance(children, list) or any(int(child) not in by_expression for child in children):
                _fail(f"{expression_path}.children", "unknown expression id")
            kind = str(expression["kind"])
            if kind in {"project", "is_variant", "variant_payload", "is_null", "load_field", "array_len", "convert"} and len(children) != 1:
                _fail(f"{expression_path}.children", f"{kind} requires one source")
            if kind in {"ref_eq", "array_get", "shl", "shr_s"} and len(children) != 2:
                _fail(f"{expression_path}.children", f"{kind} requires two operands")
            if kind in {"shl", "shr_s"}:
                count = by_expression[int(children[1])]
                if count.get("kind") != "constant" or type(count.get("value")) is not int:
                    _fail(f"{expression_path}.children[1]", "shift count must be immediate integer constant")
            if kind == "convert" and expression.get("conversion") not in CONVERSIONS:
                _fail(f"{expression_path}.conversion", "unsupported conversion")
            if kind == "load_field" and type(expression.get("field_index")) is not int:
                _fail(f"{expression_path}.field_index", "required")
            if kind == "project" and type(expression.get("index")) is not int:
                _fail(f"{expression_path}.index", "required")
            if kind in {"is_variant", "variant_payload"} and type(expression.get("variant_index")) is not int:
                _fail(f"{expression_path}.variant_index", "required")
            if kind == "variant_payload" and type(expression.get("payload_index")) is not int:
                _fail(f"{expression_path}.payload_index", "required")
        for contract_index, contract in enumerate(function["contracts"]):
            if int(contract["expression_id"]) not in by_expression:
                _fail(f"{path}.contracts[{contract_index}].expression_id", "unknown expression")
    return document


__all__ = ["CANONICAL_V3_KINDS", "SCHEMA", "VERSION", "validate_document"]
