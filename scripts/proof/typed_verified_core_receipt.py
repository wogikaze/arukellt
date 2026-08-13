"""Versioned facade for typed VerifiedCore boundary receipt validation."""

from proof import typed_verified_core_receipt_impl as _impl

SCHEMA = _impl.SCHEMA
VERSION = _impl.VERSION
TypedVerifiedCoreReceiptError = _impl.TypedVerifiedCoreReceiptError
REQUIRED_SEMANTIC_CHECKS = {
    "operator-arity-and-TypeId-preservation",
    "contract-root-typing",
    "result-return-TypeId-equality",
    "parameter-signature-local-bijection",
    "constant-payload-typing",
    "contract-kind-expression-id-uniqueness",
    "straight-line-instruction-typing",
    "acyclic-cfg-edge-typing",
    "direct-call-contract-typing",
    "recursive-call-rejection",
    "semantic-admission-before-SMT",
}
_impl.REQUIRED_SEMANTIC_CHECKS = REQUIRED_SEMANTIC_CHECKS
validate_boundary_receipt = _impl.validate_boundary_receipt

__all__ = ["REQUIRED_SEMANTIC_CHECKS", "SCHEMA", "VERSION", "TypedVerifiedCoreReceiptError", "validate_boundary_receipt"]
