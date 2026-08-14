"""Strict typed admission for proof Phase 6 machine integers."""
from __future__ import annotations
from typing import Any
from proof.machine_integer_semantics import compatibility_view, validate_machine_integer_semantics
from proof.typed_admission_v5_final import TypedVerifiedCoreError, validate_typed_document as validate_phase5

SCHEMA = "arukellt-typed-verified-core-admission"
VERSION = 6


def validate_typed_document(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        return validate_phase5(value)
    validate_machine_integer_semantics(value)
    compatibility = compatibility_view(value)
    try:
        validate_phase5(compatibility)
    except (ValueError, TypeError, KeyError) as exc:
        if isinstance(exc, TypedVerifiedCoreError):
            raise
        raise TypedVerifiedCoreError(str(exc)) from exc
    return value


__all__ = ["SCHEMA", "VERSION", "TypedVerifiedCoreError", "validate_typed_document"]
