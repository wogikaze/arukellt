"""Fail-closed proof-required release validation."""

from __future__ import annotations

from pathlib import Path
from typing import Mapping

from proof.common import load_json, sha256_file
from proof.release_payload import validate_release_payload_manifest
from proof.release_provenance import validate_release_provenance
from proof.source_proof_binding import REQUIRED_ARTIFACTS, VERSION as BINDING_VERSION, validate_binding
from proof.trust import check_release_policy, validate_trust_manifest
from proof.typed_admission_v7 import validate_typed_document as validate_v7_verified_core
from proof.typed_corehir_v3 import validate_document as validate_typed_corehir_source


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


def _require_component_version(manifest: dict[str, object], role: str, version: str) -> dict[str, object]:
    component = _trusted_component(manifest, role=role)
    if component.get("version") != version:
        raise ProofRequiredReleaseError(
            f"TrustManifest component role={role!r} must have version {version}"
        )
    return component


def _validate_phase7_chain(bound_paths: Mapping[str, Path], manifest: dict[str, object]) -> None:
    raw_source = validate_typed_corehir_source(load_json(bound_paths["typed_corehir"]))
    canonical_source = validate_typed_corehir_source(load_json(bound_paths["typed_corehir_canonical"]))
    if canonical_source.get("schema_version") != 3:
        raise ProofRequiredReleaseError("canonical TypedCoreHIR release source must be schema v3")
    profile = canonical_source.get("target_profile")
    if not isinstance(profile, dict) or (
        profile.get("integer_model"),
        profile.get("overflow"),
        profile.get("floating_point"),
    ) != ("machine", "checked", "unsupported"):
        raise ProofRequiredReleaseError("canonical TypedCoreHIR does not carry Phase 6/7 machine proof profile")

    raw_version = raw_source.get("schema_version")
    if raw_version == 1:
        if canonical_source.get("generator") != "arukellt-selfhost-v1-proof-upgrade-v2":
            raise ProofRequiredReleaseError("v1 producer source is not bound to the trusted proof v3 upgrader")
        _require_component_version(manifest, "proof-source-upgrader", "2")
        _require_component_version(manifest, "proof-source-upgrade-cli", "2")
    elif raw_version == 3:
        if sha256_file(bound_paths["typed_corehir"]) != sha256_file(bound_paths["typed_corehir_canonical"]):
            raise ProofRequiredReleaseError("native v3 raw/canonical TypedCoreHIR artifacts must be identical")
    else:
        raise ProofRequiredReleaseError(f"proof-required Phase 7 release does not admit raw TypedCoreHIR v{raw_version}")

    machine = validate_v7_verified_core(load_json(bound_paths["verified_core_machine"]))
    if machine.get("generator") != "arukellt-typed-corehir-converter-v7":
        raise ProofRequiredReleaseError("VerifiedCore machine artifact was not produced by converter v7")
    machine_profile = machine.get("target_profile")
    if not isinstance(machine_profile, dict) or (
        machine_profile.get("integer_model"),
        machine_profile.get("overflow"),
        machine_profile.get("floating_point"),
    ) != ("machine", "checked", "unsupported"):
        raise ProofRequiredReleaseError("VerifiedCore machine artifact lost the Phase 6 profile")
    memory = machine.get("proof_memory")
    if not isinstance(memory, dict) or memory.get("model") != "arukellt-readonly-heap-v1":
        raise ProofRequiredReleaseError("VerifiedCore machine artifact lost the Phase 7 memory profile")
    if sha256_file(bound_paths["verified_core_machine"]) != sha256_file(bound_paths["verified_core_normalized"]):
        raise ProofRequiredReleaseError("Phase 7 release subject must not be rewritten by the legacy mathematical normalizer")

    _require_component_version(manifest, "proof-source-converter", "7")
    _require_component_version(manifest, "proof-source-converter-cli", "7")
    _require_component_version(manifest, "typed-smt-adapter", "7")
    _require_component_version(manifest, "typed-smt-adapter-cli", "7")
    translator = manifest.get("translator")
    if not isinstance(translator, dict) or translator.get("version") != "7":
        raise ProofRequiredReleaseError("TrustManifest translator must be Phase 7")
    trust_profile = manifest.get("semantic_profile")
    if not isinstance(trust_profile, dict) or (
        trust_profile.get("integer_model"),
        trust_profile.get("overflow"),
        trust_profile.get("floating_point"),
        trust_profile.get("memory"),
    ) != ("machine", "checked", "unsupported", "read-only-heap"):
        raise ProofRequiredReleaseError("TrustManifest semantic profile is not Phase 7")


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
    if BINDING_VERSION != 5 or binding_component.get("version") != "5":
        raise ProofRequiredReleaseError(
            "TrustManifest must identify source-artifact-binding version 5"
        )

    _validate_phase7_chain(bound_paths, manifest)

    producer_path = bound_paths["producer_executable"]
    producer_digest = sha256_file(producer_path)
    producer = manifest.get("producer")
    if not isinstance(producer, dict):
        raise ProofRequiredReleaseError("TrustManifest producer must be an object")
    if producer.get("executable_sha256") != producer_digest:
        raise ProofRequiredReleaseError(
            "TrustManifest producer does not bind the supplied compiler executable"
        )

    release_binary = release_payloads.get("arukellt-wasm")
    if release_binary is None:
        raise ProofRequiredReleaseError(
            "release payload must contain the canonical arukellt-wasm artifact"
        )
    if sha256_file(release_binary) != producer_digest:
        raise ProofRequiredReleaseError(
            "authorized arukellt-wasm payload is not the proved producer executable"
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
