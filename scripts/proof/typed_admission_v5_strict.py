"""Strict typed admission through Phase 5 with independent aggregate validation."""
from __future__ import annotations
from typing import Any
from proof import verified_core_program as structural_v1
from proof import verified_core_typed_impl as typed_v1
from proof import verified_core_typed_phase4 as phase4_compat
from proof.aggregate_compat_v5 import compatibility_view as aggregate_compatibility_view
from proof.aggregate_instructions import validate_aggregate_semantics
from proof.aggregate_semantics import has_aggregates
from proof.aggregate_types import is_aggregate_type
from proof.loop_semantics import has_loops, legacy_validation_view, validate_loop_semantics
from proof.verified_core_ext import validate_document as validate_extended_document

SCHEMA = typed_v1.SCHEMA
VERSION = 5
TypedVerifiedCoreError = typed_v1.TypedVerifiedCoreError


def _function_uses_aggregate(function: dict[str, Any], aggregate_ids: set[int]) -> bool:
    if int(function["signature"]["return_type_id"]) in aggregate_ids:
        return True
    if any(int(parameter["type_id"]) in aggregate_ids for parameter in function["signature"]["parameters"]):
        return True
    if any(int(local["type_id"]) in aggregate_ids for local in function["locals"]):
        return True
    return any(
        instruction.get("op") in {"construct", "project", "is_variant", "variant_payload"}
        or any(int(argument.get("type_id", -1)) in aggregate_ids for argument in instruction.get("arguments", []))
        for block in function["body"]["blocks"]
        for instruction in block["instructions"]
    )


def _reject_out_of_phase_composition(document: dict[str, Any]) -> None:
    aggregate_ids = {int(entry["id"]) for entry in document["types"] if is_aggregate_type(entry)}
    for function_index, function in enumerate(document["functions"]):
        if not _function_uses_aggregate(function, aggregate_ids):
            continue
        if any("loop" in block for block in function["body"]["blocks"]):
            raise TypedVerifiedCoreError(f"$.functions[{function_index}].body: aggregate loops are outside proof phase 5")
        if any(instruction.get("op") == "call" for block in function["body"]["blocks"] for instruction in block["instructions"]):
            raise TypedVerifiedCoreError(f"$.functions[{function_index}].body: aggregate calls are outside proof phase 5")


def validate_typed_document(value: Any) -> dict[str, Any]:
    phase4_compat._precheck_legacy_diagnostics(value)
    document = validate_extended_document(value)
    phase4_compat._validate_call_interface_digests(document)
    checked = document
    if has_loops(document):
        validate_loop_semantics(document)
        checked = legacy_validation_view(checked)
    if has_aggregates(document):
        validate_aggregate_semantics(document)
        _reject_out_of_phase_composition(document)
        checked = aggregate_compatibility_view(checked)
    try:
        structural_v1.validate_document(checked)
        typed_v1.validate_typed_document(checked)
    except TypedVerifiedCoreError as exc:
        raise TypedVerifiedCoreError(phase4_compat._compat_message(str(exc))) from exc
    return document


__all__ = ["SCHEMA", "VERSION", "TypedVerifiedCoreError", "validate_typed_document"]
