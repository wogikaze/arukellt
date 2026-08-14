"""Strict typed admission for proof Phase 7 read-only memory."""
from __future__ import annotations
from typing import Any
from proof.readonly_memory_compat_v7 import compatibility_view
from proof.readonly_memory_semantics import validate_readonly_memory_semantics
from proof.typed_admission_v6 import TypedVerifiedCoreError, validate_typed_document as validate_phase6

SCHEMA = "arukellt-typed-verified-core-admission"
VERSION = 7


def validate_typed_document(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        return validate_phase6(value)
    validate_readonly_memory_semantics(value)
    compatibility = compatibility_view(value)
    try:
        validate_phase6(compatibility)
    except (ValueError, TypeError, KeyError) as exc:
        if isinstance(exc, TypedVerifiedCoreError):
            raise
        raise TypedVerifiedCoreError(str(exc)) from exc
    return value


__all__ = ["SCHEMA", "VERSION", "TypedVerifiedCoreError", "validate_typed_document"]
