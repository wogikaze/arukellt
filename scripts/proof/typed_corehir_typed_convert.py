"""Explicit logical type conversion facade for TypedCoreHIR proof phases 1-3."""

from proof.typed_corehir_program_convert import (
    CONVERTER,
    ExplicitTypedCoreHirError,
    SOURCE_SCHEMA,
    SOURCE_VERSION,
    convert_document,
)


def convert_typed_document(value):
    return convert_document(value)


__all__ = [
    "CONVERTER",
    "ExplicitTypedCoreHirError",
    "SOURCE_SCHEMA",
    "SOURCE_VERSION",
    "convert_typed_document",
]
