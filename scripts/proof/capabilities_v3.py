"""Validation for proof capability profile v3 (phases 0-7)."""
from __future__ import annotations
from typing import Any

SCHEMA = "arukellt-proof-capabilities"
VERSION = 3
PROFILE = "proof-phases-0-7"
EXPECTED_PHASES = {str(index) for index in range(8)}
REQUIRED_CAPABILITIES = {
    "3": {"exact-callee-interface-sha256-binding"},
    "4": {"strict-decreases-termination"},
    "5": {"arukellt-smt-datatype-v1"},
    "6": {"arukellt-machine-int-v1", "arukellt-machine-int-range-v1", "division-zero-and-min-overflow-vc"},
    "7": {"arukellt-readonly-heap-v1", "arukellt-readonly-heap-smt-v1", "nonnull-vc", "bounds-vc", "no-hidden-alias-assumptions"},
}
REQUIRED_REJECTIONS = {
    "6": {"wrapping-arithmetic", "symbolic-shift-counts", "unchecked-narrowing", "floating-point"},
    "7": {"memory-mutation", "allocation", "aggregate-contained-references", "memory-loops", "memory-calls"},
}


def _string_list(value: Any, path: str) -> list[str]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{path}: expected non-empty array")
    if any(not isinstance(item, str) or not item for item in value):
        raise ValueError(f"{path}: expected non-empty strings")
    if len(value) != len(set(value)):
        raise ValueError(f"{path}: duplicate capability/rejection")
    return value


def validate_document(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != {"schema", "schema_version", "profile", "phases"}:
        raise ValueError("$: invalid proof capability v3 document")
    if value["schema"] != SCHEMA or value["schema_version"] != VERSION or value["profile"] != PROFILE:
        raise ValueError("$: unsupported proof capability profile")
    phases = value["phases"]
    if not isinstance(phases, dict) or set(phases) != EXPECTED_PHASES:
        raise ValueError("$.phases: profile must define exactly phases 0-7")
    for phase in sorted(EXPECTED_PHASES):
        entry = phases[phase]
        if not isinstance(entry, dict) or set(entry) != {"name", "capabilities", "reject"}:
            raise ValueError(f"$.phases[{phase}]: expected name/capabilities/reject")
        if not isinstance(entry["name"], str) or not entry["name"]:
            raise ValueError(f"$.phases[{phase}].name: expected non-empty string")
        capabilities = set(_string_list(entry["capabilities"], f"$.phases[{phase}].capabilities"))
        rejects = set(_string_list(entry["reject"], f"$.phases[{phase}].reject"))
        missing_capabilities = REQUIRED_CAPABILITIES.get(phase, set()) - capabilities
        missing_rejections = REQUIRED_REJECTIONS.get(phase, set()) - rejects
        if missing_capabilities:
            raise ValueError(f"$.phases[{phase}].capabilities: missing {sorted(missing_capabilities)}")
        if missing_rejections:
            raise ValueError(f"$.phases[{phase}].reject: missing {sorted(missing_rejections)}")
    return value


__all__ = ["EXPECTED_PHASES", "PROFILE", "SCHEMA", "VERSION", "validate_document"]
