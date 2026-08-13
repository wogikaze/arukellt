"""Independent validator for typed VerifiedCore boundary evidence."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

from proof.common import sha256_file

SCHEMA = "arukellt-typed-verified-core-boundary"
VERSION = 1
_SHA256 = re.compile(r"^[0-9a-f]{64}$")
REQUIRED_SEMANTIC_CHECKS = {
    "operator-arity-and-TypeId-preservation",
    "contract-root-typing",
    "result-return-TypeId-equality",
    "parameter-signature-local-bijection",
    "constant-payload-typing",
    "global-contract-expression-id-uniqueness",
    "straight-line-instruction-typing",
    "acyclic-cfg-edge-typing",
    "direct-call-contract-typing",
    "recursive-call-rejection",
    "semantic-admission-before-SMT",
}


class TypedVerifiedCoreReceiptError(ValueError):
    pass


def validate_boundary_receipt(value: Any, *, root: Path | None = None) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise TypedVerifiedCoreReceiptError("receipt must be an object")
    expected = {"schema", "schema_version", "status", "source_schema", "target_schema", "converter", "logical_integer_metadata", "type_name_semantics", "structural_validator", "semantic_validator", "solver_adapter", "semantic_checks", "failure_action", "files"}
    if set(value) != expected:
        raise TypedVerifiedCoreReceiptError("receipt fields mismatch")
    if value.get("schema") != SCHEMA or value.get("schema_version") != VERSION:
        raise TypedVerifiedCoreReceiptError("receipt schema identity mismatch")
    required_values = {"status": "enforced", "source_schema": "arukellt-typed-corehir@1", "target_schema": "arukellt-verified-core@1", "converter": "arukellt-typed-corehir-converter-v3", "logical_integer_metadata": "explicit-bits-and-signedness", "type_name_semantics": "identity-only", "structural_validator": "verified_core.py@1", "semantic_validator": "verified_core_typed.py@2", "solver_adapter": "smtlib_typed_v1.py@1", "failure_action": "reject-before-SMT-generation"}
    for field, expected_value in required_values.items():
        if value.get(field) != expected_value:
            raise TypedVerifiedCoreReceiptError(f"{field}: expected {expected_value!r}, got {value.get(field)!r}")
    checks = value.get("semantic_checks")
    if not isinstance(checks, list) or any(not isinstance(item, str) for item in checks):
        raise TypedVerifiedCoreReceiptError("semantic_checks must be a string array")
    if set(checks) != REQUIRED_SEMANTIC_CHECKS or len(checks) != len(REQUIRED_SEMANTIC_CHECKS):
        raise TypedVerifiedCoreReceiptError("semantic_checks set mismatch")
    files = value.get("files")
    if not isinstance(files, list) or not files:
        raise TypedVerifiedCoreReceiptError("files must be a non-empty array")
    seen: set[str] = set()
    for index, entry in enumerate(files):
        if not isinstance(entry, dict) or set(entry) != {"path", "sha256"}:
            raise TypedVerifiedCoreReceiptError(f"files[{index}]: invalid entry")
        relative = entry.get("path"); digest = entry.get("sha256")
        if not isinstance(relative, str) or not relative:
            raise TypedVerifiedCoreReceiptError(f"files[{index}].path: invalid path")
        candidate = Path(relative)
        if candidate.is_absolute() or ".." in candidate.parts:
            raise TypedVerifiedCoreReceiptError(f"files[{index}].path: expected repository-relative path")
        if relative in seen:
            raise TypedVerifiedCoreReceiptError(f"files[{index}].path: duplicate path")
        seen.add(relative)
        if not isinstance(digest, str) or not _SHA256.fullmatch(digest):
            raise TypedVerifiedCoreReceiptError(f"files[{index}].sha256: invalid digest")
        if root is not None:
            path = root / candidate
            if not path.is_file():
                raise TypedVerifiedCoreReceiptError(f"files[{index}].path: file missing")
            if sha256_file(path) != digest:
                raise TypedVerifiedCoreReceiptError(f"files[{index}].sha256: digest mismatch for {relative}")
    return value


__all__ = ["REQUIRED_SEMANTIC_CHECKS", "SCHEMA", "VERSION", "TypedVerifiedCoreReceiptError", "validate_boundary_receipt"]
