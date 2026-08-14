"""Final Phase 5 TypedCoreHIR v2 aggregate converter."""
from __future__ import annotations
from typing import Any
from proof import typed_corehir_v2_convert as core
from proof.typed_admission_v5_final import validate_typed_document

CONVERTER = core.CONVERTER


def convert_document(value: Any) -> dict[str, Any]:
    previous = core.validate_typed_document
    try:
        core.validate_typed_document = validate_typed_document
        return core.convert_document(value)
    finally:
        core.validate_typed_document = previous


__all__ = ["CONVERTER", "convert_document"]
