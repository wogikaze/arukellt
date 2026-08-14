"""Typed VerifiedCore boundary receipt contract for proof Phase 5."""
from proof import typed_verified_core_receipt_impl as _impl

SCHEMA = _impl.SCHEMA
VERSION = 5
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
    "exact-callee-interface-sha256-binding",
    "recursive-call-rejection",
    "annotated-loop-invariant-typing",
    "loop-initiation-preservation-exit-vc",
    "strict-decreases-termination-vc",
    "unannotated-cycle-rejection",
    "pure-aggregate-type-metadata",
    "aggregate-constructor-projection-typing",
    "enum-variant-payload-typing",
    "deterministic-smt-datatype-v1",
    "recursive-aggregate-rejection",
    "semantic-admission-before-SMT",
}


def validate_boundary_receipt(value, root):
    previous = _impl.REQUIRED_SEMANTIC_CHECKS
    try:
        _impl.REQUIRED_SEMANTIC_CHECKS = REQUIRED_SEMANTIC_CHECKS
        return _impl.validate_boundary_receipt(value, root)
    finally:
        _impl.REQUIRED_SEMANTIC_CHECKS = previous


__all__ = ["REQUIRED_SEMANTIC_CHECKS", "SCHEMA", "VERSION", "TypedVerifiedCoreReceiptError", "validate_boundary_receipt"]
