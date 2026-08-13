"""Public fail-closed validator for VerifiedCore v1."""

from proof.verified_core_ext import SCHEMA, VERSION, validate_document

__all__ = ["SCHEMA", "VERSION", "validate_document"]
