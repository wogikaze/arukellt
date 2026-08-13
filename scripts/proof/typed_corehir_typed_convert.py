"""Explicit logical type conversion facade for TypedCoreHIR proof phases 1-3."""

from proof.typed_corehir_program_convert import (
    CONVERTER,
    ExplicitTypedCoreHirError,
    SOURCE_SCHEMA,
    SOURCE_VERSION,
    convert_document,
)


def convert_typed_document(value):
    try:
        return convert_document(value)
    except ExplicitTypedCoreHirError:
        raise
    except (ValueError, TypeError, KeyError) as exc:
        raise ExplicitTypedCoreHirError(str(exc)) from exc


__all__ = [
    "CONVERTER",
    "ExplicitTypedCoreHirError",
    "SOURCE_SCHEMA",
    "SOURCE_VERSION",
    "convert_typed_document",
]
