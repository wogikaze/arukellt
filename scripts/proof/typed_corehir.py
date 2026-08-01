"""Independent public validator for typed CoreHIR v1 artifacts."""

from __future__ import annotations

import copy
from typing import Any

from proof.common import object_value, validate_header
from proof import verified_core_expression_arena as _expression_arena

SCHEMA = "arukellt-typed-corehir"
VERSION = 1


def validate_document(value: Any) -> dict[str, Any]:
    document = object_value(value, "$")
    validate_header(document, "$", SCHEMA, VERSION)
    compatibility = copy.deepcopy(document)
    compatibility["schema"] = _expression_arena.SCHEMA
    compatibility["schema_version"] = _expression_arena.VERSION
    _expression_arena.validate_document(compatibility)
    return document


__all__ = ["SCHEMA", "VERSION", "validate_document"]
