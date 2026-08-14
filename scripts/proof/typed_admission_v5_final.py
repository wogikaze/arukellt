"""Final strict typed admission for proof Phase 5 aggregates."""
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
from proof.typed_admission_v5_strict import _reject_out_of_phase_composition
from proof.verified_core_ext import validate_document as validate_extended_document

SCHEMA = typed_v1.SCHEMA
VERSION = 5
TypedVerifiedCoreError = typed_v1.TypedVerifiedCoreError


def validate_typed_document(value: Any) -> dict[str, Any]:
    phase4_compat._precheck_legacy_diagnostics(value)
    if not isinstance(value, dict) or not has_aggregates(value):
        document = validate_extended_document(value)
        phase4_compat._validate_call_interface_digests(document)
        checked = document
        if has_loops(document):
            validate_loop_semantics(document)
            checked = legacy_validation_view(document)
        try:
            structural_v1.validate_document(checked)
            typed_v1.validate_typed_document(checked)
        except TypedVerifiedCoreError as exc:
            raise TypedVerifiedCoreError(phase4_compat._compat_message(str(exc))) from exc
        return document

    document = value
    validate_aggregate_semantics(document)
    phase4_compat._validate_call_interface_digests(document)
    _reject_out_of_phase_composition(document)
    checked = document
    if has_loops(document):
        validate_loop_semantics(document)
        checked = legacy_validation_view(document)
    checked = aggregate_compatibility_view(checked)
    try:
        structural_v1.validate_document(checked)
        typed_v1.validate_typed_document(checked)
    except TypedVerifiedCoreError as exc:
        raise TypedVerifiedCoreError(phase4_compat._compat_message(str(exc))) from exc
    return document


__all__ = ["SCHEMA", "VERSION", "TypedVerifiedCoreError", "validate_typed_document"]
