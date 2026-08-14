"""Validation for proof capability profile v2 (phases 0-5)."""
from __future__ import annotations
from typing import Any

SCHEMA = "arukellt-proof-capabilities"
VERSION = 2
EXPECTED_PHASES = {"0", "1", "2", "3", "4", "5"}


def validate_document(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != {"schema", "schema_version", "profile", "phases"}:
        raise ValueError("$: invalid proof capability v2 document")
    if value["schema"] != SCHEMA or value["schema_version"] != VERSION or value["profile"] != "proof-phases-0-5":
        raise ValueError("$: unsupported proof capability profile")
    phases = value["phases"]
    if not isinstance(phases, dict) or set(phases) != EXPECTED_PHASES:
        raise ValueError("$.phases: profile must define exactly phases 0-5")
    for phase in sorted(EXPECTED_PHASES):
        entry = phases[phase]
        if not isinstance(entry, dict) or set(entry) != {"name", "capabilities", "reject"}:
            raise ValueError(f"$.phases[{phase}]: expected name/capabilities/reject")
        if not isinstance(entry["name"], str) or not entry["name"]:
            raise ValueError(f"$.phases[{phase}].name: expected non-empty string")
        for key in ("capabilities", "reject"):
            values = entry[key]
            if not isinstance(values, list) or not values or any(not isinstance(item, str) or not item for item in values) or len(values) != len(set(values)):
                raise ValueError(f"$.phases[{phase}].{key}: expected unique non-empty strings")
    required = {
        "3": "exact-callee-interface-sha256-binding",
        "4": "strict-decreases-termination",
        "5": "arukellt-smt-datatype-v1",
    }
    for phase, token in required.items():
        if token not in phases[phase]["capabilities"]:
            raise ValueError(f"$.phases[{phase}].capabilities: missing {token}")
    return value

__all__ = ["EXPECTED_PHASES", "SCHEMA", "VERSION", "validate_document"]
