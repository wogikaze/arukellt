"""Independent public validator for typed CoreHIR v1 artifacts."""

from __future__ import annotations

import copy
from typing import Any

from proof.common import object_value, validate_header
from proof import typed_corehir_impl as _typed_corehir_impl

SCHEMA = "arukellt-typed-corehir"
VERSION = 1


def validate_document(value: Any) -> dict[str, Any]:
    document = object_value(value, "$")
    validate_header(document, "$", SCHEMA, VERSION)
    compatibility = copy.deepcopy(document)
    compatibility["schema"] = _typed_corehir_impl.SCHEMA
    compatibility["schema_version"] = _typed_corehir_impl.VERSION
    _typed_corehir_impl.validate_document(compatibility)
    return document


__all__ = ["SCHEMA", "VERSION", "validate_document"]
