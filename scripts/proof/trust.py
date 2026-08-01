"""Fail-closed validation for Arukellt proof trust artifacts.

TrustManifest records the exact producer, translator, solver, semantics, limits,
and explicit assumptions used for a solver result. ProofReceipt binds that
manifest and the source Proof IR by SHA-256. These validators intentionally use
only the Python standard library so release gates do not depend on a solver SDK.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn

from proof.ir import validate_document as validate_proof_ir

TRUST_SCHEMA = "arukellt-trust-manifest"
TRUST_SCHEMA_VERSION = 1
RECEIPT_SCHEMA = "arukellt-proof-receipt"
RECEIPT_SCHEMA_VERSION = 1
POLICY_SCHEMA = "arukellt-proof-release-policy"
POLICY_SCHEMA_VERSION = 1
PROOF_STATUSES = {"proved", "refuted", "unknown", "error"}
RELEASE_MODES = {"proof-required", "proof-optional"}


@dataclass(frozen=True)
class ValidationError(ValueError):
    path: str
    message: str

    def __str__(self) -> str:
        return f"{self.path}: {self.message}"


def _fail(path: str, message: str) -> NoReturn:
    raise ValidationError(path, message)


def _object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        _fail(path, "expected object")
    return value


def _array(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        _fail(path, "expected array")
    return value


def _string(value: Any, path: str, *, nonempty: bool = True) -> str:
    if not isinstance(value, str):
        _fail(path, "expected string")
    if nonempty and not value:
        _fail(path, "must not be empty")
    return value


def _integer(value: Any, path: str, *, minimum: int | None = None) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        _fail(path, "expected integer")
    if minimum is not None and value < minimum:
        _fail(path, f"must be >= {minimum}")
    return value


def _keys(obj: dict[str, Any], path: str, *, required: set[str], optional: set[str]) -> None:
    missing = required - obj.keys()
    if missing:
        _fail(path, f"missing field(s): {', '.join(sorted(missing))}")
    unknown = obj.keys() - required - optional
    if unknown:
        _fail(path, f"unknown field(s): {', '.join(sorted(unknown))}")


def _sha256(value: Any, path: str) -> str:
    digest = _string(value, path)
    if len(digest) != 64 or any(ch not in "0123456789abcdef" for ch in digest):
        _fail(path, "expected lowercase SHA-256 hex")
    return digest


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def _validate_tool(value: Any, path: str) -> None:
    tool = _object(value, path)
    _keys(
        tool,
        path,
        required={"name", "version", "executable_sha256"},
        optional={"arguments", "build_id"},
    )
    _string(tool["name"], f"{path}.name")
    _string(tool["version"], f"{path}.version")
    _sha256(tool["executable_sha256"], f"{path}.executable_sha256")
    if "build_id" in tool:
        _string(tool["build_id"], f"{path}.build_id")
    if "arguments" in tool:
        for index, argument in enumerate(_array(tool["arguments"], f"{path}.arguments")):
            _string(argument, f"{path}.arguments[{index}]", nonempty=False)


def _validate_semantic_profile(value: Any, path: str) -> None:
    profile = _object(value, path)
    _keys(
        profile,
        path,
        required={"integer_model", "overflow", "floating_point", "memory"},
        optional=set(),
    )
    for field in ("integer_model", "overflow", "floating_point", "memory"):
        _string(profile[field], f"{path}.{field}")


def _validate_trusted_component(value: Any, path: str) -> None:
    component = _object(value, path)
    _keys(
        component,
        path,
        required={"name", "role", "artifact_sha256"},
        optional={"version"},
    )
    _string(component["name"], f"{path}.name")
    _string(component["role"], f"{path}.role")
    _sha256(component["artifact_sha256"], f"{path}.artifact_sha256")
    if "version" in component:
        _string(component["version"], f"{path}.version")


def validate_trust_manifest(value: Any) -> dict[str, Any]:
    manifest = _object(value, "$")
    _keys(
        manifest,
        "$",
        required={
            "schema",
            "schema_version",
            "proof_ir_sha256",
            "producer",
            "translator",
            "solver",
            "semantic_profile",
            "assumptions",
            "trusted_components",
            "limits",
        },
        optional=set(),
    )
    if _string(manifest["schema"], "$.schema") != TRUST_SCHEMA:
        _fail("$.schema", f"expected {TRUST_SCHEMA!r}")
    if _integer(manifest["schema_version"], "$.schema_version") != TRUST_SCHEMA_VERSION:
        _fail("$.schema_version", f"expected {TRUST_SCHEMA_VERSION}")
    _sha256(manifest["proof_ir_sha256"], "$.proof_ir_sha256")
    _validate_tool(manifest["producer"], "$.producer")
    _validate_tool(manifest["translator"], "$.translator")
    _validate_tool(manifest["solver"], "$.solver")
    _validate_semantic_profile(manifest["semantic_profile"], "$.semantic_profile")

    assumptions = _array(manifest["assumptions"], "$.assumptions")
    for index, assumption in enumerate(assumptions):
        _string(assumption, f"$.assumptions[{index}]")
    if len(set(assumptions)) != len(assumptions):
        _fail("$.assumptions", "duplicate assumption")

    components = _array(manifest["trusted_components"], "$.trusted_components")
    if not components:
        _fail("$.trusted_components", "must record at least one trusted component")
    seen_components: set[tuple[str, str]] = set()
    for index, component_value in enumerate(components):
        path = f"$.trusted_components[{index}]"
        _validate_trusted_component(component_value, path)
        component = _object(component_value, path)
        identity = (component["name"], component["role"])
        if identity in seen_components:
            _fail(path, f"duplicate trusted component: {identity[0]} / {identity[1]}")
        seen_components.add(identity)

    limits = _object(manifest["limits"], "$.limits")
    _keys(limits, "$.limits", required={"timeout_ms", "memory_bytes"}, optional=set())
    _integer(limits["timeout_ms"], "$.limits.timeout_ms", minimum=1)
    _integer(limits["memory_bytes"], "$.limits.memory_bytes", minimum=1)
    return manifest


def _validate_obligations(value: Any, path: str) -> dict[str, int]:
    obligations = _object(value, path)
    fields = {"total", "proved", "refuted", "unknown", "errors"}
    _keys(obligations, path, required=fields, optional=set())
    counts = {
        field: _integer(obligations[field], f"{path}.{field}", minimum=0)
        for field in fields
    }
    if counts["proved"] + counts["refuted"] + counts["unknown"] + counts["errors"] != counts["total"]:
        _fail(path, "outcome counts must sum to total")
    return counts


def validate_proof_receipt(value: Any) -> dict[str, Any]:
    receipt = _object(value, "$")
    _keys(
        receipt,
        "$",
        required={
            "schema",
            "schema_version",
            "proof_ir_sha256",
            "trust_manifest_sha256",
            "solver_output_sha256",
            "status",
            "obligations",
        },
        optional={"subject"},
    )
    if _string(receipt["schema"], "$.schema") != RECEIPT_SCHEMA:
        _fail("$.schema", f"expected {RECEIPT_SCHEMA!r}")
    if _integer(receipt["schema_version"], "$.schema_version") != RECEIPT_SCHEMA_VERSION:
        _fail("$.schema_version", f"expected {RECEIPT_SCHEMA_VERSION}")
    _sha256(receipt["proof_ir_sha256"], "$.proof_ir_sha256")
    _sha256(receipt["trust_manifest_sha256"], "$.trust_manifest_sha256")
    _sha256(receipt["solver_output_sha256"], "$.solver_output_sha256")
    if "subject" in receipt:
        _string(receipt["subject"], "$.subject")

    status = _string(receipt["status"], "$.status")
    if status not in PROOF_STATUSES:
        _fail("$.status", f"unsupported status: {status}")
    counts = _validate_obligations(receipt["obligations"], "$.obligations")
    if counts["total"] < 1:
        _fail("$.obligations.total", "proof receipt must cover at least one obligation")
    if status == "proved" and not (
        counts["proved"] == counts["total"]
        and counts["refuted"] == 0
        and counts["unknown"] == 0
        and counts["errors"] == 0
    ):
        _fail("$.status", "proved requires every obligation to be proved")
    if status == "refuted" and counts["refuted"] == 0:
        _fail("$.status", "refuted requires at least one refuted obligation")
    if status == "unknown" and counts["unknown"] == 0:
        _fail("$.status", "unknown requires at least one unknown obligation")
    if status == "error" and counts["errors"] == 0:
        _fail("$.status", "error requires at least one errored obligation")
    return receipt


def validate_bound_receipt(
    proof_ir_path: Path,
    trust_manifest_path: Path,
    receipt_path: Path,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    proof_ir = validate_proof_ir(load_json(proof_ir_path))
    manifest = validate_trust_manifest(load_json(trust_manifest_path))
    receipt = validate_proof_receipt(load_json(receipt_path))

    proof_ir_digest = sha256_file(proof_ir_path)
    manifest_digest = sha256_file(trust_manifest_path)
    if manifest["proof_ir_sha256"] != proof_ir_digest:
        _fail("$.proof_ir_sha256", "TrustManifest does not bind the supplied Proof IR")
    if receipt["proof_ir_sha256"] != proof_ir_digest:
        _fail("$.proof_ir_sha256", "ProofReceipt does not bind the supplied Proof IR")
    if receipt["trust_manifest_sha256"] != manifest_digest:
        _fail("$.trust_manifest_sha256", "ProofReceipt does not bind the supplied TrustManifest")
    return proof_ir, manifest, receipt


def _validate_policy_artifact(value: Any, path: str) -> None:
    artifact = _object(value, path)
    _keys(
        artifact,
        path,
        required={"proof_ir", "trust_manifest", "receipt"},
        optional={"label"},
    )
    for field in ("proof_ir", "trust_manifest", "receipt"):
        candidate = Path(_string(artifact[field], f"{path}.{field}"))
        if candidate.is_absolute() or ".." in candidate.parts:
            _fail(f"{path}.{field}", "must be a relative path without '..'")
    if "label" in artifact:
        _string(artifact["label"], f"{path}.label")


def validate_release_policy(value: Any) -> dict[str, Any]:
    policy = _object(value, "$")
    _keys(
        policy,
        "$",
        required={"schema", "schema_version", "mode", "artifacts"},
        optional=set(),
    )
    if _string(policy["schema"], "$.schema") != POLICY_SCHEMA:
        _fail("$.schema", f"expected {POLICY_SCHEMA!r}")
    if _integer(policy["schema_version"], "$.schema_version") != POLICY_SCHEMA_VERSION:
        _fail("$.schema_version", f"expected {POLICY_SCHEMA_VERSION}")
    mode = _string(policy["mode"], "$.mode")
    if mode not in RELEASE_MODES:
        _fail("$.mode", f"unsupported mode: {mode}")
    artifacts = _array(policy["artifacts"], "$.artifacts")
    if mode == "proof-required" and not artifacts:
        _fail("$.artifacts", "proof-required release must list at least one receipt")
    for index, artifact in enumerate(artifacts):
        _validate_policy_artifact(artifact, f"$.artifacts[{index}]")
    return policy


def check_release_policy(policy_path: Path) -> tuple[str, int]:
    policy = validate_release_policy(load_json(policy_path))
    base = policy_path.parent.resolve()
    for index, artifact in enumerate(policy["artifacts"]):
        paths: dict[str, Path] = {}
        for field in ("proof_ir", "trust_manifest", "receipt"):
            path = (base / artifact[field]).resolve()
            if base not in path.parents and path != base:
                _fail(f"$.artifacts[{index}].{field}", "resolved path escapes policy directory")
            if not path.is_file():
                _fail(f"$.artifacts[{index}].{field}", f"missing file: {path}")
            paths[field] = path
        _, _, receipt = validate_bound_receipt(
            paths["proof_ir"], paths["trust_manifest"], paths["receipt"]
        )
        if policy["mode"] == "proof-required" and receipt["status"] != "proved":
            _fail(f"$.artifacts[{index}].receipt", "proof-required release requires status=proved")
    return policy["mode"], len(policy["artifacts"])
