"""Public fail-closed validator for VerifiedCore v1."""

from __future__ import annotations

from typing import Any

from proof.common import ValidationError
from proof.verified_core_ext import SCHEMA, VERSION, validate_document as _validate_document


def _public_message(message: str) -> str:
    if message.startswith("missing fields:"):
        return "missing field(s):" + message[len("missing fields:"):]
    if message == "return type mismatch":
        return "return type must match signature"
    return message


def validate_document(value: Any) -> dict[str, Any]:
    try:
        return _validate_document(value)
    except ValidationError:
        raise
    except ValueError as exc:
        text = str(exc)
        path, separator, message = text.partition(": ")
        if not separator:
            path, message = "$", text
        raise ValidationError(path, _public_message(message)) from exc


__all__ = ["SCHEMA", "VERSION", "validate_document"]
