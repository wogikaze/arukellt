"""Independent validation for the solver TrustManifest boundary receipt."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

from proof.common import sha256_file

SCHEMA = "arukellt-solver-trust-boundary"
VERSION = 1
REQUIRED_CAPABILITIES = {
    "embedded-trust-manifest",
    "embedded-proof-receipt",
    "subject-binding",
    "solver-input-binding",
    "toolchain-binding",
    "solver-output-binding",
    "process-outcome-consistency",
    "independent-result-validation",
    "production-bypass-rejection",
}
REQUIRED_PRODUCERS = {
    "scripts/run/run-proof-solver.py",
    "scripts/gen/write-solver-receipts.py",
}
_SHA256 = re.compile(r"^[0-9a-f]{64}$")


class SolverTrustBoundaryReceiptError(ValueError):
    """The solver trust boundary receipt is malformed or stale."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise SolverTrustBoundaryReceiptError(message)


def validate_solver_trust_boundary_receipt(
    value: object,
    *,
    root: Path,
) -> dict[str, Any]:
    _require(isinstance(value, dict), "receipt must be an object")
    receipt = value
    expected = {
        "schema",
        "schema_version",
        "status",
        "primary_result",
        "raw_solver_output_role",
        "trust_manifest_policy",
        "proof_receipt_policy",
        "capabilities",
        "public_producers",
        "failure_action",
        "files",
    }
    _require(set(receipt) == expected, "receipt field set mismatch")
    _require(receipt["schema"] == SCHEMA, f"schema must be {SCHEMA}")
    _require(receipt["schema_version"] == VERSION, f"schema_version must be {VERSION}")
    _require(receipt["status"] == "enforced", "status must be enforced")
    _require(receipt["primary_result"] == "arukellt-solver-result@1", "primary result mismatch")
    _require(receipt["raw_solver_output_role"] == "evidence-only", "raw output must be evidence-only")
    _require(receipt["trust_manifest_policy"] == "embedded-and-file-bound", "TrustManifest policy mismatch")
    _require(receipt["proof_receipt_policy"] == "embedded-and-file-bound", "ProofReceipt policy mismatch")
    _require(receipt["failure_action"] == "no-valid-solver-result", "failure action mismatch")

    capabilities = receipt["capabilities"]
    _require(isinstance(capabilities, list), "capabilities must be an array")
    _require(set(capabilities) == REQUIRED_CAPABILITIES, "capability set mismatch")
    _require(len(capabilities) == len(REQUIRED_CAPABILITIES), "duplicate capability")

    producers = receipt["public_producers"]
    _require(isinstance(producers, list), "public_producers must be an array")
    _require(set(producers) == REQUIRED_PRODUCERS, "public producer set mismatch")
    _require(len(producers) == len(REQUIRED_PRODUCERS), "duplicate public producer")

    files = receipt["files"]
    _require(isinstance(files, list) and files, "files must be a non-empty array")
    seen: set[str] = set()
    resolved_root = root.resolve()
    for index, raw in enumerate(files):
        _require(isinstance(raw, dict), f"files[{index}] must be an object")
        _require(set(raw) == {"path", "sha256"}, f"files[{index}] field set mismatch")
        relative = raw["path"]
        digest = raw["sha256"]
        _require(isinstance(relative, str) and relative, f"files[{index}].path invalid")
        path_value = Path(relative)
        _require(not path_value.is_absolute() and ".." not in path_value.parts, f"files[{index}].path escapes root")
        _require(relative not in seen, f"duplicate path: {relative}")
        seen.add(relative)
        _require(isinstance(digest, str) and _SHA256.fullmatch(digest) is not None, f"files[{index}].sha256 invalid")
        resolved = (resolved_root / path_value).resolve()
        _require(resolved == resolved_root or resolved_root in resolved.parents, f"files[{index}].path escapes root")
        _require(resolved.is_file(), f"receipt file missing: {relative}")
        _require(sha256_file(resolved) == digest, f"receipt digest mismatch: {relative}")
    return receipt


__all__ = [
    "REQUIRED_CAPABILITIES",
    "REQUIRED_PRODUCERS",
    "SCHEMA",
    "VERSION",
    "SolverTrustBoundaryReceiptError",
    "validate_solver_trust_boundary_receipt",
]
