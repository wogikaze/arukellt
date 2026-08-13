"""Versioned validation receipt for the pinned major-boundary registry."""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from proof.boundary_registry import (
    BoundaryFileSource,
    BoundaryRegistryError,
    iter_file_references,
    load_registry,
    sha256_bytes,
    sha256_file,
    validate_registry_files,
)

SCHEMA = "arukellt-boundary-registry-validation"
VERSION = 1
_SHA256 = re.compile(r"^[0-9a-f]{64}$")


class BoundaryRegistryReceiptError(ValueError):
    """A boundary-registry validation receipt is malformed or stale."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise BoundaryRegistryReceiptError(message)


def _file_key(value: dict[str, Any]) -> tuple[str, str]:
    return str(value["commit"]), str(value["path"])


def create_validation_receipt(
    registry_path: Path,
    source: BoundaryFileSource,
) -> dict[str, Any]:
    registry = load_registry(registry_path)
    fetched = validate_registry_files(registry, source)
    boundaries = [
        {
            "id": boundary["id"],
            "schema": boundary["artifact"]["schema"],
            "schema_version": boundary["artifact"]["schema_version"],
            "kind": boundary["artifact"]["kind"],
        }
        for boundary in registry["boundaries"]
    ]
    files = [
        {
            "commit": commit,
            "path": path,
            "sha256": sha256_bytes(data),
            "size_bytes": len(data),
        }
        for (commit, path), data in sorted(fetched.items())
    ]
    return {
        "schema": SCHEMA,
        "schema_version": VERSION,
        "status": "validated",
        "repository": registry["repository"],
        "registry_sha256": sha256_file(registry_path),
        "boundary_count": len(boundaries),
        "boundaries": boundaries,
        "files": files,
    }


def write_validation_receipt(value: dict[str, Any], output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(output.name + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(output)


def validate_validation_receipt(
    value: object,
    *,
    registry_path: Path,
    source: BoundaryFileSource | None = None,
) -> dict[str, Any]:
    _require(isinstance(value, dict), "receipt must be an object")
    receipt = value
    expected_fields = {
        "schema",
        "schema_version",
        "status",
        "repository",
        "registry_sha256",
        "boundary_count",
        "boundaries",
        "files",
    }
    _require(set(receipt) == expected_fields, "receipt field set mismatch")
    _require(receipt["schema"] == SCHEMA, f"receipt schema must be {SCHEMA}")
    _require(receipt["schema_version"] == VERSION, f"receipt schema_version must be {VERSION}")
    _require(receipt["status"] == "validated", "receipt status must be validated")

    registry = load_registry(registry_path)
    _require(receipt["repository"] == registry["repository"], "receipt repository mismatch")
    _require(
        receipt["registry_sha256"] == sha256_file(registry_path),
        "receipt does not bind the supplied registry",
    )

    expected_boundaries = [
        {
            "id": boundary["id"],
            "schema": boundary["artifact"]["schema"],
            "schema_version": boundary["artifact"]["schema_version"],
            "kind": boundary["artifact"]["kind"],
        }
        for boundary in registry["boundaries"]
    ]
    _require(receipt["boundary_count"] == len(expected_boundaries), "boundary count mismatch")
    _require(receipt["boundaries"] == expected_boundaries, "boundary inventory mismatch")

    files = receipt["files"]
    _require(isinstance(files, list) and bool(files), "receipt files must be a non-empty array")
    by_key: dict[tuple[str, str], dict[str, Any]] = {}
    for index, raw in enumerate(files):
        _require(isinstance(raw, dict), f"files[{index}] must be an object")
        _require(
            set(raw) == {"commit", "path", "sha256", "size_bytes"},
            f"files[{index}] field set mismatch",
        )
        commit = raw["commit"]
        path = raw["path"]
        digest = raw["sha256"]
        size = raw["size_bytes"]
        _require(isinstance(commit, str) and len(commit) == 40, f"files[{index}].commit invalid")
        _require(isinstance(path, str) and bool(path), f"files[{index}].path invalid")
        _require(isinstance(digest, str) and _SHA256.fullmatch(digest) is not None, f"files[{index}].sha256 invalid")
        _require(type(size) is int and size > 0, f"files[{index}].size_bytes invalid")
        key = (commit, path)
        _require(key not in by_key, f"duplicate receipt file: {commit}:{path}")
        by_key[key] = raw

    expected_keys = {
        (str(ref["commit"]), str(ref["path"]))
        for _, ref in iter_file_references(registry)
    }
    _require(set(by_key) == expected_keys, "receipt file reference set mismatch")

    if source is not None:
        try:
            fetched = validate_registry_files(registry, source)
        except BoundaryRegistryError as exc:
            raise BoundaryRegistryReceiptError(str(exc)) from exc
        for key, data in fetched.items():
            entry = by_key[key]
            _require(entry["sha256"] == sha256_bytes(data), f"stale receipt digest: {key[0]}:{key[1]}")
            _require(entry["size_bytes"] == len(data), f"stale receipt size: {key[0]}:{key[1]}")
    return receipt


def load_validation_receipt(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise BoundaryRegistryReceiptError(f"invalid receipt JSON: {exc}") from exc
    _require(isinstance(value, dict), "receipt must be an object")
    return value


__all__ = [
    "BoundaryRegistryReceiptError",
    "SCHEMA",
    "VERSION",
    "create_validation_receipt",
    "load_validation_receipt",
    "validate_validation_receipt",
    "write_validation_receipt",
]
