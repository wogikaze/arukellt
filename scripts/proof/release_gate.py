"""Fail-closed proof-required release validation."""

from __future__ import annotations

from pathlib import Path
from typing import Mapping

from proof.common import load_json, sha256_file
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


def _trusted_component_digest(
    manifest: dict[str, object],
    *,
    role: str,
) -> str:
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
    digest = matching[0].get("artifact_sha256")
    if not isinstance(digest, str):
        raise ProofRequiredReleaseError(
            f"TrustManifest component role={role!r} has no artifact_sha256"
        )
    return digest


def validate_proof_required_release(
    policy_path: Path,
    source_binding_path: Path,
    bound_paths: Mapping[str, Path],
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

    policy = load_json(policy_path)
    if not isinstance(policy, dict):
        raise ProofRequiredReleaseError("release policy must be an object")
    artifacts = policy.get("artifacts")
    if not isinstance(artifacts, list) or len(artifacts) != 1:
        raise ProofRequiredReleaseError(
            "source-bound proof-required release currently requires exactly one artifact entry"
        )
    artifact = artifacts[0]
    if not isinstance(artifact, dict):
        raise ProofRequiredReleaseError("release artifact entry must be an object")

    policy_base = policy_path.parent.resolve()
    manifest_path = _resolve_policy_file(
        policy_base,
        artifact.get("trust_manifest"),
        "release artifact trust_manifest",
    )
    manifest = validate_trust_manifest(load_json(manifest_path))

    binding_digest = sha256_file(source_binding_path)
    recorded_binding_digest = _trusted_component_digest(
        manifest,
        role="source-artifact-binding",
    )
    if recorded_binding_digest != binding_digest:
        raise ProofRequiredReleaseError(
            "TrustManifest does not bind the supplied source proof binding"
        )

    producer_path = bound_paths["producer_executable"]
    producer = manifest.get("producer")
    if not isinstance(producer, dict):
        raise ProofRequiredReleaseError("TrustManifest producer must be an object")
    if producer.get("executable_sha256") != sha256_file(producer_path):
        raise ProofRequiredReleaseError(
            "TrustManifest producer does not bind the supplied compiler executable"
        )

    return mode, artifact_count


__all__ = [
    "ProofRequiredReleaseError",
    "validate_proof_required_release",
]
