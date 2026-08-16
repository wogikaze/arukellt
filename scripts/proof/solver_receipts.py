"""Generate TrustManifest and ProofReceipt artifacts from solver output."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from proof.common import load_json, sha256_file
from proof.trust import validate_bound_proof, validate_proof_receipt, validate_trust_manifest
from proof.verified_core import validate_document as validate_verified_core

TOOLCHAIN_SCHEMA = "arukellt-proof-toolchain"
TOOLCHAIN_VERSION = 1

_PROFILE_REQUIRED = {"integer_model", "overflow", "floating_point", "memory"}
_PROFILE_STRING_OPTIONAL = {
    "aggregate_model",
    "aggregate_encoding",
    "machine_integer_model",
    "machine_integer_encoding",
    "memory_model",
    "memory_encoding",
    "capability_profile",
}
_PROFILE_INT_OPTIONAL = {"source_schema_version", "raw_source_schema_version"}
_PROFILE_BOOL_OPTIONAL = {"source_upgrade_active", "phase67_available", "phase67_active"}
_PROFILE_OPTIONAL = _PROFILE_STRING_OPTIONAL | _PROFILE_INT_OPTIONAL | _PROFILE_BOOL_OPTIONAL

_STATUS_MAP = {
    "proved": "proved",
    "unsat": "proved",
    "refuted": "refuted",
    "sat": "refuted",
    "unknown": "unknown",
    "error": "error",
}


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{label}: expected object")
    return value


def _require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{label}: expected non-empty string")
    return value


def _require_positive_int(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise ValueError(f"{label}: expected positive integer")
    return value


def _exact_keys(value: dict[str, Any], label: str, required: set[str], optional: set[str] = set()) -> None:
    missing = required - value.keys()
    if missing:
        raise ValueError(f"{label}: missing field(s): {', '.join(sorted(missing))}")
    unknown = value.keys() - required - optional
    if unknown:
        raise ValueError(f"{label}: unknown field(s): {', '.join(sorted(unknown))}")


def _resolve_file(base: Path, raw: Any, label: str) -> Path:
    value = _require_string(raw, label)
    candidate = Path(value)
    if candidate.is_absolute() or ".." in candidate.parts:
        raise ValueError(f"{label}: expected relative path without '..'")
    resolved = (base / candidate).resolve()
    if resolved != base and base not in resolved.parents:
        raise ValueError(f"{label}: resolved path escapes toolchain directory")
    if not resolved.is_file():
        raise ValueError(f"{label}: missing file: {resolved}")
    return resolved


def _tool(value: Any, base: Path, label: str) -> dict[str, Any]:
    tool = _require_object(value, label)
    _exact_keys(tool, label, {"name", "version", "executable"}, {"arguments", "build_id"})
    path = _resolve_file(base, tool["executable"], f"{label}.executable")
    result: dict[str, Any] = {
        "name": _require_string(tool["name"], f"{label}.name"),
        "version": _require_string(tool["version"], f"{label}.version"),
        "executable_sha256": sha256_file(path),
    }
    if "build_id" in tool:
        result["build_id"] = _require_string(tool["build_id"], f"{label}.build_id")
    if "arguments" in tool:
        arguments = tool["arguments"]
        if not isinstance(arguments, list) or any(not isinstance(item, str) for item in arguments):
            raise ValueError(f"{label}.arguments: expected string array")
        result["arguments"] = arguments
    return result


def _semantic_profile(value: Any) -> dict[str, Any]:
    profile = _require_object(value, "toolchain.semantic_profile")
    _exact_keys(profile, "toolchain.semantic_profile", _PROFILE_REQUIRED, _PROFILE_OPTIONAL)
    rendered: dict[str, Any] = {
        field: _require_string(profile[field], f"toolchain.semantic_profile.{field}")
        for field in sorted(_PROFILE_REQUIRED)
    }
    for field in sorted(_PROFILE_STRING_OPTIONAL):
        if field in profile:
            rendered[field] = _require_string(profile[field], f"toolchain.semantic_profile.{field}")
    for field in sorted(_PROFILE_INT_OPTIONAL):
        if field in profile:
            raw = profile[field]
            if type(raw) is not int or raw < 1:
                raise ValueError(f"toolchain.semantic_profile.{field}: expected positive integer")
            rendered[field] = raw
    for field in sorted(_PROFILE_BOOL_OPTIONAL):
        if field in profile:
            raw = profile[field]
            if type(raw) is not bool:
                raise ValueError(f"toolchain.semantic_profile.{field}: expected boolean")
            rendered[field] = raw
    return rendered


def load_toolchain(path: Path) -> dict[str, Any]:
    value = _require_object(load_json(path), "toolchain")
    _exact_keys(
        value,
        "toolchain",
        {
            "schema",
            "schema_version",
            "producer",
            "translator",
            "solver",
            "semantic_profile",
            "assumptions",
            "trusted_components",
            "limits",
        },
    )
    if value["schema"] != TOOLCHAIN_SCHEMA:
        raise ValueError(f"toolchain.schema: expected {TOOLCHAIN_SCHEMA!r}")
    if value["schema_version"] != TOOLCHAIN_VERSION:
        raise ValueError(f"toolchain.schema_version: expected {TOOLCHAIN_VERSION}")
    return value


def parse_solver_output(path: Path) -> dict[str, int | str]:
    counts = {"proved": 0, "refuted": 0, "unknown": 0, "errors": 0}
    for line_number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith(";") or line == "success":
            continue
        token = line
        if ":" in token:
            prefix, suffix = token.rsplit(":", 1)
            if prefix.strip() and suffix.strip():
                token = suffix.strip()
        normalized = _STATUS_MAP.get(token.lower())
        if normalized is None:
            if token.lower().startswith("(error"):
                normalized = "error"
            else:
                raise ValueError(f"solver output line {line_number}: unsupported result {line!r}")
        if normalized == "error":
            counts["errors"] += 1
        else:
            counts[normalized] += 1
    total = counts["proved"] + counts["refuted"] + counts["unknown"] + counts["errors"]
    if total == 0:
        raise ValueError("solver output contains no obligations")
    if counts["errors"]:
        status = "error"
    elif counts["refuted"]:
        status = "refuted"
    elif counts["unknown"]:
        status = "unknown"
    else:
        status = "proved"
    return {"total": total, **counts, "status": status}


def _subject_ref(subject_path: Path) -> dict[str, Any]:
    subject = validate_verified_core(load_json(subject_path))
    return {
        "schema": subject["schema"],
        "schema_version": subject["schema_version"],
        "sha256": sha256_file(subject_path),
    }


def build_trust_manifest(subject_path: Path, toolchain_path: Path) -> dict[str, Any]:
    toolchain = load_toolchain(toolchain_path)
    base = toolchain_path.parent.resolve()
    semantic_profile = _semantic_profile(toolchain["semantic_profile"])

    assumptions = toolchain["assumptions"]
    if not isinstance(assumptions, list) or any(not isinstance(item, str) or not item for item in assumptions):
        raise ValueError("toolchain.assumptions: expected non-empty string array")
    if len(set(assumptions)) != len(assumptions):
        raise ValueError("toolchain.assumptions: duplicate assumption")

    components_raw = toolchain["trusted_components"]
    if not isinstance(components_raw, list) or not components_raw:
        raise ValueError("toolchain.trusted_components: expected non-empty array")
    components: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()
    for index, raw in enumerate(components_raw):
        label = f"toolchain.trusted_components[{index}]"
        component = _require_object(raw, label)
        _exact_keys(component, label, {"name", "role", "artifact"}, {"version"})
        name = _require_string(component["name"], f"{label}.name")
        role = _require_string(component["role"], f"{label}.role")
        if (name, role) in seen:
            raise ValueError(f"{label}: duplicate trusted component")
        seen.add((name, role))
        artifact_path = _resolve_file(base, component["artifact"], f"{label}.artifact")
        rendered: dict[str, Any] = {
            "name": name,
            "role": role,
            "artifact_sha256": sha256_file(artifact_path),
        }
        if "version" in component:
            rendered["version"] = _require_string(component["version"], f"{label}.version")
        components.append(rendered)

    limits = _require_object(toolchain["limits"], "toolchain.limits")
    _exact_keys(limits, "toolchain.limits", {"timeout_ms", "memory_bytes"})

    manifest = {
        "schema": "arukellt-trust-manifest",
        "schema_version": 1,
        "subject": _subject_ref(subject_path),
        "producer": _tool(toolchain["producer"], base, "toolchain.producer"),
        "translator": _tool(toolchain["translator"], base, "toolchain.translator"),
        "solver": _tool(toolchain["solver"], base, "toolchain.solver"),
        "semantic_profile": semantic_profile,
        "assumptions": assumptions,
        "trusted_components": components,
        "limits": {
            "timeout_ms": _require_positive_int(limits["timeout_ms"], "toolchain.limits.timeout_ms"),
            "memory_bytes": _require_positive_int(limits["memory_bytes"], "toolchain.limits.memory_bytes"),
        },
    }
    return validate_trust_manifest(manifest)


def build_proof_receipt(
    subject_path: Path,
    trust_manifest_path: Path,
    solver_output_path: Path,
) -> dict[str, Any]:
    parsed = parse_solver_output(solver_output_path)
    receipt = {
        "schema": "arukellt-proof-receipt",
        "schema_version": 1,
        "subject": _subject_ref(subject_path),
        "trust_manifest_sha256": sha256_file(trust_manifest_path),
        "solver_output_sha256": sha256_file(solver_output_path),
        "status": parsed["status"],
        "obligations": {
            "total": parsed["total"],
            "proved": parsed["proved"],
            "refuted": parsed["refuted"],
            "unknown": parsed["unknown"],
            "errors": parsed["errors"],
        },
    }
    return validate_proof_receipt(receipt)


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def generate_solver_receipts(
    subject_path: Path,
    solver_output_path: Path,
    toolchain_path: Path,
    trust_manifest_path: Path,
    proof_receipt_path: Path,
) -> tuple[dict[str, Any], dict[str, Any]]:
    manifest = build_trust_manifest(subject_path, toolchain_path)
    write_json(trust_manifest_path, manifest)
    receipt = build_proof_receipt(subject_path, trust_manifest_path, solver_output_path)
    write_json(proof_receipt_path, receipt)
    validate_bound_proof(subject_path, trust_manifest_path, proof_receipt_path, solver_output_path)
    return manifest, receipt


__all__ = [
    "TOOLCHAIN_SCHEMA",
    "TOOLCHAIN_VERSION",
    "build_proof_receipt",
    "build_trust_manifest",
    "generate_solver_receipts",
    "load_toolchain",
    "parse_solver_output",
]
