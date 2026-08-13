"""Fail-closed validation for trust, proof, translation, and release artifacts."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from proof.common import (
    ValidationError,
    array_value,
    bool_value,
    exact_keys,
    fail,
    int_value,
    load_json,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
    validate_header,
)
from proof.verified_core import SCHEMA as VERIFIED_CORE_SCHEMA
from proof.verified_core import validate_document as validate_verified_core

TRUST_SCHEMA = "arukellt-trust-manifest"
TRUST_VERSION = 1
PROOF_SCHEMA = "arukellt-proof-receipt"
PROOF_VERSION = 1
TRANSLATION_SCHEMA = "arukellt-translation-validation"
TRANSLATION_VERSION = 1
POLICY_SCHEMA = "arukellt-proof-release-policy"
POLICY_VERSION = 1

HARD_GATES = {
    "versioned_boundary_artifacts",
    "explicit_backend_type_abi_layout",
    "typed_verified_core_emission",
    "optimizer_translation_validation",
    "solver_trust_manifest",
    "legacy_mutable_table_removed",
    "proof_receipt_release_enforced",
}
PROOF_STATUSES = {"proved", "refuted", "unknown", "error"}
TRANSLATION_STATUSES = {"validated", "failed", "unknown", "error"}
RELEASE_MODES = {"proof-optional", "proof-required"}


def _artifact_ref(value: Any, path: str) -> dict[str, Any]:
    ref = object_value(value, path)
    exact_keys(ref, path, required={"schema", "schema_version", "sha256"})
    string_value(ref["schema"], f"{path}.schema")
    int_value(ref["schema_version"], f"{path}.schema_version", minimum=1)
    sha256_value(ref["sha256"], f"{path}.sha256")
    return ref


def _tool(value: Any, path: str) -> None:
    tool = object_value(value, path)
    exact_keys(
        tool,
        path,
        required={"name", "version", "executable_sha256"},
        optional={"arguments", "build_id"},
    )
    string_value(tool["name"], f"{path}.name")
    string_value(tool["version"], f"{path}.version")
    sha256_value(tool["executable_sha256"], f"{path}.executable_sha256")
    if "build_id" in tool:
        string_value(tool["build_id"], f"{path}.build_id")
    if "arguments" in tool:
        for index, argument in enumerate(array_value(tool["arguments"], f"{path}.arguments")):
            string_value(argument, f"{path}.arguments[{index}]", nonempty=False)


def validate_trust_manifest(value: Any) -> dict[str, Any]:
    manifest = object_value(value, "$")
    exact_keys(
        manifest,
        "$",
        required={
            "schema",
            "schema_version",
            "subject",
            "producer",
            "translator",
            "solver",
            "semantic_profile",
            "assumptions",
            "trusted_components",
            "limits",
        },
    )
    validate_header(manifest, "$", TRUST_SCHEMA, TRUST_VERSION)
    _artifact_ref(manifest["subject"], "$.subject")
    _tool(manifest["producer"], "$.producer")
    _tool(manifest["translator"], "$.translator")
    _tool(manifest["solver"], "$.solver")

    profile = object_value(manifest["semantic_profile"], "$.semantic_profile")
    exact_keys(
        profile,
        "$.semantic_profile",
        required={"integer_model", "overflow", "floating_point", "memory"},
    )
    for field in ("integer_model", "overflow", "floating_point", "memory"):
        string_value(profile[field], f"$.semantic_profile.{field}")

    assumptions = array_value(manifest["assumptions"], "$.assumptions")
    for index, assumption in enumerate(assumptions):
        string_value(assumption, f"$.assumptions[{index}]")
    if len(set(assumptions)) != len(assumptions):
        fail("$.assumptions", "duplicate assumption")

    components = array_value(manifest["trusted_components"], "$.trusted_components")
    if not components:
        fail("$.trusted_components", "must record at least one trusted component")
    seen: set[tuple[str, str]] = set()
    for index, raw in enumerate(components):
        path = f"$.trusted_components[{index}]"
        component = object_value(raw, path)
        exact_keys(
            component,
            path,
            required={"name", "role", "artifact_sha256"},
            optional={"version"},
        )
        name = string_value(component["name"], f"{path}.name")
        role = string_value(component["role"], f"{path}.role")
        if (name, role) in seen:
            fail(path, f"duplicate trusted component: {name} / {role}")
        seen.add((name, role))
        sha256_value(component["artifact_sha256"], f"{path}.artifact_sha256")
        if "version" in component:
            string_value(component["version"], f"{path}.version")

    limits = object_value(manifest["limits"], "$.limits")
    exact_keys(limits, "$.limits", required={"timeout_ms", "memory_bytes"})
    int_value(limits["timeout_ms"], "$.limits.timeout_ms", minimum=1)
    int_value(limits["memory_bytes"], "$.limits.memory_bytes", minimum=1)
    return manifest


def _counts(value: Any, path: str, outcomes: tuple[str, ...]) -> dict[str, int]:
    obj = object_value(value, path)
    exact_keys(obj, path, required={"total", *outcomes})
    result = {key: int_value(obj[key], f"{path}.{key}", minimum=0) for key in {"total", *outcomes}}
    if result["total"] < 1:
        fail(f"{path}.total", "must cover at least one obligation")
    if sum(result[key] for key in outcomes) != result["total"]:
        fail(path, "outcome counts must sum to total")
    return result


def validate_proof_receipt(value: Any) -> dict[str, Any]:
    receipt = object_value(value, "$")
    exact_keys(
        receipt,
        "$",
        required={
            "schema",
            "schema_version",
            "subject",
            "trust_manifest_sha256",
            "solver_output_sha256",
            "status",
            "obligations",
        },
    )
    validate_header(receipt, "$", PROOF_SCHEMA, PROOF_VERSION)
    _artifact_ref(receipt["subject"], "$.subject")
    sha256_value(receipt["trust_manifest_sha256"], "$.trust_manifest_sha256")
    sha256_value(receipt["solver_output_sha256"], "$.solver_output_sha256")
    status = string_value(receipt["status"], "$.status")
    if status not in PROOF_STATUSES:
        fail("$.status", f"unsupported status: {status}")
    counts = _counts(receipt["obligations"], "$.obligations", ("proved", "refuted", "unknown", "errors"))
    if status == "proved" and not (
        counts["proved"] == counts["total"]
        and counts["refuted"] == counts["unknown"] == counts["errors"] == 0
    ):
        fail("$.status", "proved requires every obligation to be proved")
    if status == "refuted" and counts["refuted"] == 0:
        fail("$.status", "refuted requires at least one refuted obligation")
    if status == "unknown" and counts["unknown"] == 0:
        fail("$.status", "unknown requires at least one unknown obligation")
    if status == "error" and counts["errors"] == 0:
        fail("$.status", "error requires at least one errored obligation")
    return receipt


def validate_translation_receipt(value: Any) -> dict[str, Any]:
    receipt = object_value(value, "$")
    exact_keys(
        receipt,
        "$",
        required={
            "schema",
            "schema_version",
            "pass",
            "source",
            "target",
            "validator",
            "trust_manifest_sha256",
            "status",
            "obligations",
        },
    )
    validate_header(receipt, "$", TRANSLATION_SCHEMA, TRANSLATION_VERSION)
    pass_info = object_value(receipt["pass"], "$.pass")
    exact_keys(pass_info, "$.pass", required={"name", "version", "configuration_sha256"})
    string_value(pass_info["name"], "$.pass.name")
    string_value(pass_info["version"], "$.pass.version")
    sha256_value(pass_info["configuration_sha256"], "$.pass.configuration_sha256")
    _artifact_ref(receipt["source"], "$.source")
    _artifact_ref(receipt["target"], "$.target")
    _tool(receipt["validator"], "$.validator")
    sha256_value(receipt["trust_manifest_sha256"], "$.trust_manifest_sha256")
    status = string_value(receipt["status"], "$.status")
    if status not in TRANSLATION_STATUSES:
        fail("$.status", f"unsupported status: {status}")
    counts = _counts(receipt["obligations"], "$.obligations", ("validated", "failed", "unknown", "errors"))
    if status == "validated" and not (
        counts["validated"] == counts["total"]
        and counts["failed"] == counts["unknown"] == counts["errors"] == 0
    ):
        fail("$.status", "validated requires every obligation to be validated")
    if status == "failed" and counts["failed"] == 0:
        fail("$.status", "failed requires at least one failed obligation")
    if status == "unknown" and counts["unknown"] == 0:
        fail("$.status", "unknown requires at least one unknown obligation")
    if status == "error" and counts["errors"] == 0:
        fail("$.status", "error requires at least one errored obligation")
    return receipt


def _relative_path(value: Any, path: str) -> str:
    raw = string_value(value, path)
    candidate = Path(raw)
    if candidate.is_absolute() or ".." in candidate.parts:
        fail(path, "must be a relative path without '..'")
    return raw


def validate_release_policy(value: Any) -> dict[str, Any]:
    policy = object_value(value, "$")
    exact_keys(
        policy,
        "$",
        required={"schema", "schema_version", "mode", "hard_gates", "artifacts"},
    )
    validate_header(policy, "$", POLICY_SCHEMA, POLICY_VERSION)
    mode = string_value(policy["mode"], "$.mode")
    if mode not in RELEASE_MODES:
        fail("$.mode", f"unsupported mode: {mode}")

    gates = object_value(policy["hard_gates"], "$.hard_gates")
    exact_keys(gates, "$.hard_gates", required=HARD_GATES)
    for gate in HARD_GATES:
        bool_value(gates[gate], f"$.hard_gates.{gate}")
    if mode == "proof-required":
        incomplete = sorted(gate for gate, satisfied in gates.items() if not satisfied)
        if incomplete:
            fail("$.hard_gates", f"proof-required release has unmet gate(s): {', '.join(incomplete)}")

    artifacts = array_value(policy["artifacts"], "$.artifacts")
    if mode == "proof-required" and not artifacts:
        fail("$.artifacts", "proof-required release must list at least one proved receipt")
    for index, raw in enumerate(artifacts):
        path = f"$.artifacts[{index}]"
        artifact = object_value(raw, path)
        exact_keys(
            artifact,
            path,
            required={
                "label",
                "subject",
                "trust_manifest",
                "receipt",
                "solver_output",
                "translation_receipts",
            },
        )
        string_value(artifact["label"], f"{path}.label")
        for field in ("subject", "trust_manifest", "receipt", "solver_output"):
            _relative_path(artifact[field], f"{path}.{field}")
        for tr_index, tr_raw in enumerate(
            array_value(artifact["translation_receipts"], f"{path}.translation_receipts")
        ):
            tr_path = f"{path}.translation_receipts[{tr_index}]"
            tr = object_value(tr_raw, tr_path)
            exact_keys(tr, tr_path, required={"receipt", "source", "target"})
            for field in ("receipt", "source", "target"):
                _relative_path(tr[field], f"{tr_path}.{field}")
    return policy


def _resolve(base: Path, raw: str, path: str) -> Path:
    candidate = (base / raw).resolve()
    if candidate != base and base not in candidate.parents:
        fail(path, "resolved path escapes policy directory")
    if not candidate.is_file():
        fail(path, f"missing file: {candidate}")
    return candidate


def _subject(path: Path) -> dict[str, Any]:
    value = load_json(path)
    schema = object_value(value, "$").get("schema")
    if schema == VERIFIED_CORE_SCHEMA:
        return validate_verified_core(value)
    fail("$.schema", f"unsupported proof subject schema: {schema!r}")


def _ref(document: dict[str, Any], path: Path) -> dict[str, Any]:
    return {
        "schema": document["schema"],
        "schema_version": document["schema_version"],
        "sha256": sha256_file(path),
    }


def validate_bound_proof(
    subject_path: Path,
    trust_manifest_path: Path,
    receipt_path: Path,
    solver_output_path: Path,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    subject = _subject(subject_path)
    manifest = validate_trust_manifest(load_json(trust_manifest_path))
    receipt = validate_proof_receipt(load_json(receipt_path))
    expected_ref = _ref(subject, subject_path)
    if manifest["subject"] != expected_ref:
        fail("$.subject", "TrustManifest does not bind the supplied subject artifact")
    if receipt["subject"] != expected_ref:
        fail("$.subject", "ProofReceipt does not bind the supplied subject artifact")
    if receipt["trust_manifest_sha256"] != sha256_file(trust_manifest_path):
        fail("$.trust_manifest_sha256", "ProofReceipt does not bind the supplied TrustManifest")
    if receipt["solver_output_sha256"] != sha256_file(solver_output_path):
        fail("$.solver_output_sha256", "ProofReceipt does not bind the supplied solver output")
    return subject, manifest, receipt


def validate_bound_translation(
    source_path: Path,
    target_path: Path,
    trust_manifest_path: Path,
    receipt_path: Path,
) -> dict[str, Any]:
    source = _subject(source_path)
    target = _subject(target_path)
    validate_trust_manifest(load_json(trust_manifest_path))
    receipt = validate_translation_receipt(load_json(receipt_path))
    if receipt["source"] != _ref(source, source_path):
        fail("$.source", "translation receipt does not bind the supplied source artifact")
    if receipt["target"] != _ref(target, target_path):
        fail("$.target", "translation receipt does not bind the supplied target artifact")
    if receipt["trust_manifest_sha256"] != sha256_file(trust_manifest_path):
        fail("$.trust_manifest_sha256", "translation receipt does not bind the supplied TrustManifest")
    return receipt


def check_release_policy(policy_path: Path) -> tuple[str, int]:
    policy = validate_release_policy(load_json(policy_path))
    base = policy_path.parent.resolve()
    for index, artifact in enumerate(policy["artifacts"]):
        prefix = f"$.artifacts[{index}]"
        manifest_path = _resolve(base, artifact["trust_manifest"], f"{prefix}.trust_manifest")
        _, _, receipt = validate_bound_proof(
            _resolve(base, artifact["subject"], f"{prefix}.subject"),
            manifest_path,
            _resolve(base, artifact["receipt"], f"{prefix}.receipt"),
            _resolve(base, artifact["solver_output"], f"{prefix}.solver_output"),
        )
        if policy["mode"] == "proof-required" and receipt["status"] != "proved":
            fail(f"{prefix}.receipt", "proof-required release requires status=proved")
        for tr_index, tr in enumerate(artifact["translation_receipts"]):
            tr_prefix = f"{prefix}.translation_receipts[{tr_index}]"
            tr_receipt = validate_bound_translation(
                _resolve(base, tr["source"], f"{tr_prefix}.source"),
                _resolve(base, tr["target"], f"{tr_prefix}.target"),
                manifest_path,
                _resolve(base, tr["receipt"], f"{tr_prefix}.receipt"),
            )
            if policy["mode"] == "proof-required" and tr_receipt["status"] != "validated":
                fail(f"{tr_prefix}.receipt", "proof-required release requires status=validated")
    return policy["mode"], len(policy["artifacts"])


__all__ = [
    "ValidationError",
    "check_release_policy",
    "validate_bound_proof",
    "validate_bound_translation",
    "validate_proof_receipt",
    "validate_release_policy",
    "validate_translation_receipt",
    "validate_trust_manifest",
]
