"""Versioned provenance for a proof-authorized release invocation."""

from __future__ import annotations

import re
from typing import Any

SCHEMA = "arukellt-release-provenance"
VERSION = 1
_COMMIT_SHA = re.compile(r"^[0-9a-f]{40}$")
_REF_TYPES = {"tag", "branch", "pull_request"}


class ReleaseProvenanceError(ValueError):
    """Release provenance is malformed or does not match the active release."""


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ReleaseProvenanceError(f"{label}: expected non-empty string")
    return value


def _commit(value: object, label: str) -> str:
    text = _text(value, label).lower()
    if not _COMMIT_SHA.fullmatch(text):
        raise ReleaseProvenanceError(f"{label}: expected full 40-character commit SHA")
    return text


def create_release_provenance(
    *,
    repository: str,
    commit_sha: str,
    ref_type: str,
    ref_name: str,
    workflow: str,
    run_id: str,
) -> dict[str, object]:
    document = {
        "schema": SCHEMA,
        "schema_version": VERSION,
        "repository": repository,
        "commit_sha": commit_sha,
        "ref_type": ref_type,
        "ref_name": ref_name,
        "workflow": workflow,
        "run_id": run_id,
    }
    return validate_release_provenance(document)


def validate_release_provenance(
    value: object,
    *,
    expected_repository: str | None = None,
    expected_commit: str | None = None,
    expected_tag: str | None = None,
) -> dict[str, object]:
    if not isinstance(value, dict):
        raise ReleaseProvenanceError("release provenance must be an object")
    expected_keys = {
        "schema",
        "schema_version",
        "repository",
        "commit_sha",
        "ref_type",
        "ref_name",
        "workflow",
        "run_id",
    }
    if set(value) != expected_keys:
        raise ReleaseProvenanceError(
            f"release provenance fields mismatch: {sorted(value)}"
        )
    if value.get("schema") != SCHEMA or value.get("schema_version") != VERSION:
        raise ReleaseProvenanceError("release provenance schema identity mismatch")

    repository = _text(value.get("repository"), "repository")
    if repository.count("/") != 1 or any(ch.isspace() for ch in repository):
        raise ReleaseProvenanceError("repository: expected owner/name")
    commit_sha = _commit(value.get("commit_sha"), "commit_sha")
    ref_type = _text(value.get("ref_type"), "ref_type")
    if ref_type not in _REF_TYPES:
        raise ReleaseProvenanceError(f"ref_type: unsupported value {ref_type!r}")
    ref_name = _text(value.get("ref_name"), "ref_name")
    if any(ch.isspace() for ch in ref_name):
        raise ReleaseProvenanceError("ref_name: whitespace is not allowed")
    _text(value.get("workflow"), "workflow")
    _text(value.get("run_id"), "run_id")

    if expected_repository is not None and repository != expected_repository:
        raise ReleaseProvenanceError(
            f"repository mismatch: expected {expected_repository}, got {repository}"
        )
    if expected_commit is not None and commit_sha != _commit(expected_commit, "expected_commit"):
        raise ReleaseProvenanceError(
            f"commit mismatch: expected {expected_commit.lower()}, got {commit_sha}"
        )
    if expected_tag is not None:
        if ref_type != "tag":
            raise ReleaseProvenanceError(
                f"proof-required release requires ref_type='tag', got {ref_type!r}"
            )
        if ref_name != expected_tag:
            raise ReleaseProvenanceError(
                f"tag mismatch: expected {expected_tag}, got {ref_name}"
            )
        if not ref_name.startswith("v"):
            raise ReleaseProvenanceError("proof-required release tag must start with 'v'")
    return value


__all__ = [
    "SCHEMA",
    "VERSION",
    "ReleaseProvenanceError",
    "create_release_provenance",
    "validate_release_provenance",
]
