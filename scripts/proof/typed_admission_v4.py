"""Typed proof admission through Phase 4."""
from __future__ import annotations
from typing import Any
from proof import verified_core_typed_impl as impl
from proof import verified_core_typed_phase4 as compat
from proof.loop_semantics import has_loops, legacy_validation_view, validate_loop_semantics
from proof.verified_core import validate_document

SCHEMA = impl.SCHEMA
VERSION = 4
TypedVerifiedCoreError = impl.TypedVerifiedCoreError

def validate_typed_document(value: Any) -> dict[str, Any]:
    compat._precheck_legacy_diagnostics(value)
    document = validate_document(value)
    compat._validate_call_interface_digests(document)
    checked = document
    if has_loops(document):
        validate_loop_semantics(document)
        checked = legacy_validation_view(document)
    try:
        impl.validate_typed_document(checked)
    except TypedVerifiedCoreError as exc:
        raise TypedVerifiedCoreError(compat._compat_message(str(exc))) from exc
    return document

__all__ = ["SCHEMA", "VERSION", "TypedVerifiedCoreError", "validate_typed_document"]
