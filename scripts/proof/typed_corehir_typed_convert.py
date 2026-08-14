"""Explicit logical type conversion facade for TypedCoreHIR proof phases 1-4."""

from proof.typed_corehir_phase4_convert import (
    CONVERTER,
    ExplicitTypedCoreHirError,
    SOURCE_SCHEMA,
    SOURCE_VERSION,
    convert_document,
)
from proof.verified_core_interface import allow_unbound_call_interfaces, bind_call_interfaces
from proof.verified_core_typed import validate_typed_document


def _compat_message(message: str) -> str:
    if "integer type requires explicit bits and signed fields" in message:
        return "expected 32 or 64: " + message
    if "must match value_type" in message:
        return "representation mismatch: " + message
    return message


def convert_typed_document(value):
    try:
        with allow_unbound_call_interfaces():
            result = convert_document(value)
        bind_call_interfaces(result)
        return validate_typed_document(result)
    except ExplicitTypedCoreHirError:
        raise
    except (ValueError, TypeError, KeyError) as exc:
        raise ExplicitTypedCoreHirError(_compat_message(str(exc))) from exc


__all__ = [
    "CONVERTER",
    "ExplicitTypedCoreHirError",
    "SOURCE_SCHEMA",
    "SOURCE_VERSION",
    "convert_typed_document",
]
