"""Public fail-closed validator for VerifiedCore v1.

The implementation lives in ``verified_core_program`` so CFG/body semantics can
expand without changing this import surface used by proof receipts and tools.
"""

from proof.verified_core_program import SCHEMA, VERSION, validate_document

__all__ = ["SCHEMA", "VERSION", "validate_document"]
