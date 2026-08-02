"""Fail-closed proof-required release validation."""

from __future__ import annotations

from pathlib import Path
from typing import Mapping

from proof.common import load_json, sha256_file
from proof.release_payload import validate_release_payload_manifest
from proof.release_provenance import validate_release_provenance
from proof.source_proof_binding import REQUIRED_ARTIFACTS, validate_binding
from proof.trust import check_release_policy, validate_trust_manifest


class ProofRequiredReleaseError(ValueError):
    """The release proof chain is incomplete, stale, or not hash-bound."""


def _resolve_policy_file(base: Path, raw: object, label: str) -> Path:
    if not isinstance(raw, str) or not raw:
        raise ProofRequiredReleaseError(f"{label}: expected relative path")
    candidate = Path(raw)
    if candidate.is_absolute() or ".." in candidate.parts:
        raise ProofRequiredReleaseError(f"{label}: expected relative path without '..'")
    resolved = (base / candidate).resolve()
    if resolved != base and base not in resolved.parents:
        raise ProofRequiredReleaseError(f"{label}: path escapes policy directory")
    if not resolved.is_file():
        raise ProofRequiredReleaseError(f"{label}: file not found: {resolved}")
    return resolved


def primary_release_artifact_paths(policy_path: Path) -> dict[str, Path]:
    policy = load_json(policy_path)
    if not isinstance(policy, dict):
        raise ProofRequiredReleaseError("release policy must be an object")
    artifacts = policy.get("artifacts")
    if not isinstance(artifacts, list) or len(artifacts) != 1:
        raise ProofRequiredReleaseError(
            "source-bound proof-required release requires exactly one artifact entry"
        )
    artifact = artifacts[0]
    if not isinstance(artifact, dict):
        raise ProofRequiredReleaseError("release artifact entry must be an object")
    base = policy_path.parent.resolve()
    return {
        field: _resolve_policy_file(base, artifact.get(field), f"release artifact {field}")
        for field in ("subject", "trust_manifest", "receipt", "solver_output")
    }


def _trusted_component(
    manifest: dict[str, object],
    *,
    role: str,
) -> dict[str, object]:
    components = manifest.get("trusted_components")
    if not isinstance(components, list):
        raise ProofRequiredReleaseError("TrustManifest trusted_components must be an array")
    matching = [
        component
        for component in components
        if isinstance(component, dict) and component.get("role") == role
    ]
    if len(matching) != 1:
        raise ProofRequiredReleaseError(
            f"TrustManifest must contain exactly one component with role={role!r}"
        )
    return matching[0]


def validate_proof_required_release(
    policy_path: Path,
    source_binding_path: Path,
    bound_paths: Mapping[str, Path],
    *,
    expected_repository: str,
    expected_commit: str,
    expected_tag: str,
    release_payloads: Mapping[str, Path],
) -> tuple[str, int]:
    mode, artifact_count = check_release_policy(policy_path)
    if mode != "proof-required":
        raise ProofRequiredReleaseError(
            f"release policy mode must be proof-required, found {mode!r}"
        )

    missing = [name for name in REQUIRED_ARTIFACTS if name not in bound_paths]
    if missing:
        raise ProofRequiredReleaseError(f"missing source binding path(s): {missing}")
    if not source_binding_path.is_file():
        raise ProofRequiredReleaseError(
            f"source proof binding file not found: {source_binding_path}"
        )
    validate_binding(load_json(source_binding_path), bound_paths)

    validate_release_provenance(
        load_json(bound_paths["release_provenance"]),
        expected_repository=expected_repository,
        expected_commit=expected_commit,
        expected_tag=expected_tag,
    )
    validate_release_payload_manifest(
        load_json(bound_paths["release_payload_manifest"]),
        release_payloads,
    )

    artifact_paths = primary_release_artifact_paths(policy_path)
    manifest = validate_trust_manifest(load_json(artifact_paths["trust_manifest"]))

    binding_component = _trusted_component(
        manifest,
        role="source-artifact-binding",
    )
    binding_digest = sha256_file(source_binding_path)
    if binding_component.get("artifact_sha256") != binding_digest:
        raise ProofRequiredReleaseError(
            "TrustManifest does not bind the supplied source proof binding"
        )
    if binding_component.get("version") != "3":
        raise ProofRequiredReleaseError(
            "TrustManifest must identify source-artifact-binding version 3"
        )

    producer_path = bound_paths["producer_executable"]
    producer = manifest.get("producer")
    if not isinstance(producer, dict):
        raise ProofRequiredReleaseError("TrustManifest producer must be an object")
    if producer.get("executable_sha256") != sha256_file(producer_path):
        raise ProofRequiredReleaseError(
            "TrustManifest producer does not bind the supplied compiler executable"
        )

    normalized_subject = bound_paths["verified_core_normalized"]
    normalized_digest = sha256_file(normalized_subject)
    manifest_subject = manifest.get("subject")
    if not isinstance(manifest_subject, dict):
        raise ProofRequiredReleaseError("TrustManifest subject must be an object")
    if manifest_subject.get("sha256") != normalized_digest:
        raise ProofRequiredReleaseError(
            "TrustManifest proof subject is not the normalized VerifiedCore in the source binding"
        )
    if sha256_file(artifact_paths["subject"]) != normalized_digest:
        raise ProofRequiredReleaseError(
            "release policy subject is not the normalized VerifiedCore in the source binding"
        )

    return mode, artifact_count


__all__ = [
    "ProofRequiredReleaseError",
    "primary_release_artifact_paths",
    "validate_proof_required_release",
]
