"""Versioned solver result carrying its TrustManifest and ProofReceipt.

The raw solver stream is evidence, not the primary result.  A valid result
contains the exact validated TrustManifest and ProofReceipt objects and binds
them to the subject, solver input, toolchain, and captured output files.
"""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

from proof.common import load_json, sha256_file
from proof.trust import (
    validate_bound_proof,
    validate_proof_receipt,
    validate_trust_manifest,
)

SCHEMA = "arukellt-solver-result"
VERSION = 1
EXECUTION_MODES = {"solver-process", "captured-output"}
PROOF_STATUSES = {"proved", "refuted", "unknown", "error"}


class SolverResultError(ValueError):
    """The solver result is malformed, incomplete, or stale."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise SolverResultError(message)


def _sha256(value: object, path: str) -> str:
    _require(
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value),
        f"{path}: expected lowercase SHA-256",
    )
    return str(value)


def _exact_keys(value: object, expected: set[str], path: str) -> dict[str, Any]:
    _require(isinstance(value, dict), f"{path}: expected object")
    document = value
    _require(set(document) == expected, f"{path}: field set mismatch")
    return document


def _execution(value: object, *, status: str) -> dict[str, Any]:
    execution = _exact_keys(
        value,
        {"mode", "returncode", "timed_out"},
        "$.execution",
    )
    mode = execution["mode"]
    _require(mode in EXECUTION_MODES, f"$.execution.mode: unsupported mode {mode!r}")
    _require(type(execution["timed_out"]) is bool, "$.execution.timed_out: expected boolean")

    if mode == "solver-process":
        returncode = execution["returncode"]
        _require(type(returncode) is int, "$.execution.returncode: expected integer")
        if execution["timed_out"]:
            _require(returncode == 124, "$.execution: timeout must use returncode 124")
            _require(status == "error", "$.execution: timeout must produce status=error")
        if returncode != 0:
            _require(status == "error", "$.execution: nonzero solver exit must produce status=error")
        if status == "proved":
            _require(returncode == 0, "$.execution: proved result requires solver exit 0")
    else:
        _require(execution["returncode"] is None, "$.execution.returncode: captured output requires null")
        _require(execution["timed_out"] is False, "$.execution: captured output cannot claim timeout")
    return execution


def validate_solver_result(
    value: object,
    *,
    subject_path: Path,
    solver_input_path: Path,
    toolchain_path: Path,
    solver_output_path: Path,
    trust_manifest_path: Path,
    proof_receipt_path: Path,
) -> dict[str, Any]:
    document = _exact_keys(
        value,
        {
            "schema",
            "schema_version",
            "subject",
            "toolchain_sha256",
            "solver_input_sha256",
            "solver_output_sha256",
            "execution",
            "status",
            "obligations",
            "trust_manifest",
            "proof_receipt",
        },
        "$",
    )
    _require(document["schema"] == SCHEMA, f"$.schema: expected {SCHEMA!r}")
    _require(document["schema_version"] == VERSION, f"$.schema_version: expected {VERSION}")

    status = document["status"]
    _require(status in PROOF_STATUSES, f"$.status: unsupported status {status!r}")
    _execution(document["execution"], status=str(status))

    _, external_manifest, external_receipt = validate_bound_proof(
        subject_path,
        trust_manifest_path,
        proof_receipt_path,
        solver_output_path,
    )
    embedded_manifest = validate_trust_manifest(document["trust_manifest"])
    embedded_receipt = validate_proof_receipt(document["proof_receipt"])
    _require(
        embedded_manifest == external_manifest,
        "$.trust_manifest: embedded manifest differs from supplied TrustManifest",
    )
    _require(
        embedded_receipt == external_receipt,
        "$.proof_receipt: embedded receipt differs from supplied ProofReceipt",
    )

    _require(document["subject"] == external_manifest["subject"], "$.subject: manifest subject mismatch")
    _require(document["subject"] == external_receipt["subject"], "$.subject: receipt subject mismatch")
    _require(document["status"] == external_receipt["status"], "$.status: receipt status mismatch")
    _require(
        document["obligations"] == external_receipt["obligations"],
        "$.obligations: receipt obligation counts mismatch",
    )

    expected_digests = {
        "toolchain_sha256": sha256_file(toolchain_path),
        "solver_input_sha256": sha256_file(solver_input_path),
        "solver_output_sha256": sha256_file(solver_output_path),
    }
    for field, expected in expected_digests.items():
        actual = _sha256(document[field], f"$.{field}")
        _require(actual == expected, f"$.{field}: digest mismatch")
    _require(
        document["solver_output_sha256"] == external_receipt["solver_output_sha256"],
        "$.solver_output_sha256: ProofReceipt binds another solver output",
    )
    return document


def create_solver_result(
    *,
    subject_path: Path,
    solver_input_path: Path,
    toolchain_path: Path,
    solver_output_path: Path,
    trust_manifest_path: Path,
    proof_receipt_path: Path,
    execution_mode: str,
    process_returncode: int | None,
    timed_out: bool,
) -> dict[str, Any]:
    _, manifest, receipt = validate_bound_proof(
        subject_path,
        trust_manifest_path,
        proof_receipt_path,
        solver_output_path,
    )
    document = {
        "schema": SCHEMA,
        "schema_version": VERSION,
        "subject": copy.deepcopy(manifest["subject"]),
        "toolchain_sha256": sha256_file(toolchain_path),
        "solver_input_sha256": sha256_file(solver_input_path),
        "solver_output_sha256": sha256_file(solver_output_path),
        "execution": {
            "mode": execution_mode,
            "returncode": process_returncode,
            "timed_out": timed_out,
        },
        "status": receipt["status"],
        "obligations": copy.deepcopy(receipt["obligations"]),
        "trust_manifest": copy.deepcopy(manifest),
        "proof_receipt": copy.deepcopy(receipt),
    }
    return validate_solver_result(
        document,
        subject_path=subject_path,
        solver_input_path=solver_input_path,
        toolchain_path=toolchain_path,
        solver_output_path=solver_output_path,
        trust_manifest_path=trust_manifest_path,
        proof_receipt_path=proof_receipt_path,
    )


def write_solver_result(document: dict[str, Any], output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def validate_solver_result_file(
    result_path: Path,
    *,
    subject_path: Path,
    solver_input_path: Path,
    toolchain_path: Path,
    solver_output_path: Path,
    trust_manifest_path: Path,
    proof_receipt_path: Path,
) -> dict[str, Any]:
    return validate_solver_result(
        load_json(result_path),
        subject_path=subject_path,
        solver_input_path=solver_input_path,
        toolchain_path=toolchain_path,
        solver_output_path=solver_output_path,
        trust_manifest_path=trust_manifest_path,
        proof_receipt_path=proof_receipt_path,
    )


__all__ = [
    "EXECUTION_MODES",
    "SCHEMA",
    "VERSION",
    "SolverResultError",
    "create_solver_result",
    "validate_solver_result",
    "validate_solver_result_file",
    "write_solver_result",
]
