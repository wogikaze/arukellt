"""Independent validator for the proof capability/profile manifest."""

from __future__ import annotations

from typing import Any

SCHEMA = "arukellt-proof-capabilities"
VERSION = 1
_REQUIRED_PHASE_FEATURES = {
    "0": {"typed-contract-expressions", "single-return-body", "int-bool-types", "source-toolchain-receipt-binding"},
    "1": {"straight-line-instructions", "copy", "unary", "binary", "division-side-conditions"},
    "2": {"acyclic-cfg", "conditional-branch", "goto", "block-parameters", "multiple-returns", "path-conditions"},
    "3": {"direct-contract-calls", "callee-requires-obligations", "callee-ensures-assumptions", "non-recursive-call-graph"},
}
_REQUIRED_UNSUPPORTED = {"cyclic-cfg", "recursion", "indirect-calls", "mutable-memory", "heap-aliasing", "floating-point", "machine-integer-equivalence"}


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate_capabilities(value: Any) -> dict[str, Any]:
    _require(isinstance(value, dict), "$: expected object")
    _require(set(value) == {"schema", "schema_version", "verified_core_schema", "semantic_profile", "phases", "unsupported_until_later_phase", "rules"}, "$: unsupported or missing fields")
    _require(value["schema"] == SCHEMA, "$.schema: unsupported schema")
    _require(value["schema_version"] == VERSION, "$.schema_version: unsupported version")
    _require(value["verified_core_schema"] == 1, "$.verified_core_schema: expected v1")
    _require(value["semantic_profile"] == {"integer_model": "mathematical", "overflow": "checked", "floating_point": "unsupported", "memory": "pure-values"}, "$.semantic_profile: unsupported proof semantics")
    phases = value["phases"]
    _require(isinstance(phases, dict) and set(phases) == set(_REQUIRED_PHASE_FEATURES), "$.phases: expected phases 0..3")
    for phase, required in _REQUIRED_PHASE_FEATURES.items():
        entry = phases[phase]
        expected_status = "enforced" if phase == "0" else "enabled"
        _require(isinstance(entry, dict) and set(entry) == {"status", "features"}, f"$.phases.{phase}: invalid fields")
        _require(entry["status"] == expected_status, f"$.phases.{phase}.status: expected {expected_status}")
        features = entry["features"]
        _require(isinstance(features, list) and all(isinstance(item, str) and item for item in features), f"$.phases.{phase}.features: expected strings")
        _require(len(features) == len(set(features)), f"$.phases.{phase}.features: duplicate feature")
        _require(set(features) >= required, f"$.phases.{phase}.features: required capability missing")
    unsupported = value["unsupported_until_later_phase"]
    _require(isinstance(unsupported, list) and set(unsupported) >= _REQUIRED_UNSUPPORTED, "$.unsupported_until_later_phase: required fail-closed feature missing")
    _require(value["rules"] == {"unsupported_constructs": "reject-before-smt", "semantic_expansion": "manifest-and-validator-first", "source_profile_normalization": "reject-machine-arithmetic", "proof_receipts": "source-subject-toolchain-bound"}, "$.rules: proof expansion rules changed")
    return value


__all__ = ["SCHEMA", "VERSION", "validate_capabilities"]
