"""Authorization artifact emitted only after the complete proof release gate passes."""

from __future__ import annotations

import json
import re
from pathlib import Path

from proof.common import load_json, sha256_file

SCHEMA = "arukellt-proof-release-authorization"
VERSION = 1
_SHA256 = re.compile(r"^[0-9a-f]{64}$")


class ReleaseAuthorizationError(ValueError):
    """A release authorization artifact is malformed or stale."""


def create_release_authorization(
    *,
    repository: str,
    commit_sha: str,
    tag: str,
    policy_path: Path,
    source_binding_path: Path,
    trust_manifest_path: Path,
    proof_receipt_path: Path,
    payload_manifest_path: Path,
) -> dict[str, object]:
    return {
        "schema": SCHEMA,
        "schema_version": VERSION,
        "status": "authorized",
        "repository": repository,
        "commit_sha": commit_sha.lower(),
        "tag": tag,
        "policy_sha256": sha256_file(policy_path),
        "source_binding_sha256": sha256_file(source_binding_path),
        "trust_manifest_sha256": sha256_file(trust_manifest_path),
        "proof_receipt_sha256": sha256_file(proof_receipt_path),
        "release_payload_manifest_sha256": sha256_file(payload_manifest_path),
    }


def write_release_authorization(document: dict[str, object], output: Path) -> None:
    validate_release_authorization(document)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def validate_release_authorization(value: object) -> dict[str, object]:
    if not isinstance(value, dict):
        raise ReleaseAuthorizationError("release authorization must be an object")
    expected = {
        "schema",
        "schema_version",
        "status",
        "repository",
        "commit_sha",
        "tag",
        "policy_sha256",
        "source_binding_sha256",
        "trust_manifest_sha256",
        "proof_receipt_sha256",
        "release_payload_manifest_sha256",
    }
    if set(value) != expected:
        raise ReleaseAuthorizationError("release authorization fields mismatch")
    if value.get("schema") != SCHEMA or value.get("schema_version") != VERSION:
        raise ReleaseAuthorizationError("release authorization schema identity mismatch")
    if value.get("status") != "authorized":
        raise ReleaseAuthorizationError("release authorization status must be authorized")
    for field in ("repository", "commit_sha", "tag"):
        if not isinstance(value.get(field), str) or not value[field]:
            raise ReleaseAuthorizationError(f"{field}: expected non-empty string")
    commit = str(value["commit_sha"])
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ReleaseAuthorizationError("commit_sha: expected full commit SHA")
    for field in (
        "policy_sha256",
        "source_binding_sha256",
        "trust_manifest_sha256",
        "proof_receipt_sha256",
        "release_payload_manifest_sha256",
    ):
        digest = value.get(field)
        if not isinstance(digest, str) or not _SHA256.fullmatch(digest):
            raise ReleaseAuthorizationError(f"{field}: expected SHA-256")
    return value


def validate_bound_release_authorization(
    authorization_path: Path,
    *,
    repository: str,
    commit_sha: str,
    tag: str,
    policy_path: Path,
    source_binding_path: Path,
    trust_manifest_path: Path,
    proof_receipt_path: Path,
    payload_manifest_path: Path,
) -> dict[str, object]:
    authorization = validate_release_authorization(load_json(authorization_path))
    expected_identity = {
        "repository": repository,
        "commit_sha": commit_sha.lower(),
        "tag": tag,
    }
    for field, expected in expected_identity.items():
        if authorization.get(field) != expected:
            raise ReleaseAuthorizationError(
                f"{field} mismatch: expected {expected}, got {authorization.get(field)}"
            )
    bound_files = {
        "policy_sha256": policy_path,
        "source_binding_sha256": source_binding_path,
        "trust_manifest_sha256": trust_manifest_path,
        "proof_receipt_sha256": proof_receipt_path,
        "release_payload_manifest_sha256": payload_manifest_path,
    }
    for field, path in bound_files.items():
        if not path.is_file():
            raise ReleaseAuthorizationError(f"{field}: file not found: {path}")
        digest = sha256_file(path)
        if authorization.get(field) != digest:
            raise ReleaseAuthorizationError(
                f"{field} mismatch: expected {digest}, got {authorization.get(field)}"
            )
    return authorization


__all__ = [
    "SCHEMA",
    "VERSION",
    "ReleaseAuthorizationError",
    "create_release_authorization",
    "validate_bound_release_authorization",
    "validate_release_authorization",
    "write_release_authorization",
]
