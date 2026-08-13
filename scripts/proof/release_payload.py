"""Versioned manifest binding the exact files authorized for release."""

from __future__ import annotations

from pathlib import Path
from typing import Mapping

from proof.common import sha256_file

SCHEMA = "arukellt-release-payload-manifest"
VERSION = 1


class ReleasePayloadError(ValueError):
    """The release payload manifest is missing, stale, or ambiguous."""


def create_release_payload_manifest(
    payloads: Mapping[str, Path],
) -> dict[str, object]:
    if not payloads:
        raise ReleasePayloadError("release payload must contain at least one file")
    entries: list[dict[str, object]] = []
    seen_paths: set[Path] = set()
    for name in sorted(payloads):
        if not name or any(ch.isspace() for ch in name):
            raise ReleasePayloadError(f"invalid payload name: {name!r}")
        path = payloads[name]
        if not path.is_file():
            raise ReleasePayloadError(f"{name}: file not found: {path}")
        resolved = path.resolve()
        if resolved in seen_paths:
            raise ReleasePayloadError(f"duplicate payload path: {path}")
        seen_paths.add(resolved)
        entries.append(
            {
                "name": name,
                "sha256": sha256_file(path),
                "size_bytes": path.stat().st_size,
            }
        )
    return {
        "schema": SCHEMA,
        "schema_version": VERSION,
        "artifacts": entries,
    }


def validate_release_payload_manifest(
    value: object,
    payloads: Mapping[str, Path],
) -> dict[str, object]:
    if not isinstance(value, dict):
        raise ReleasePayloadError("release payload manifest must be an object")
    if set(value) != {"schema", "schema_version", "artifacts"}:
        raise ReleasePayloadError("release payload manifest fields mismatch")
    if value.get("schema") != SCHEMA or value.get("schema_version") != VERSION:
        raise ReleasePayloadError("release payload manifest schema identity mismatch")
    entries = value.get("artifacts")
    if not isinstance(entries, list) or not entries:
        raise ReleasePayloadError("release payload manifest must list artifacts")

    by_name: dict[str, dict[str, object]] = {}
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != {"name", "sha256", "size_bytes"}:
            raise ReleasePayloadError("invalid release payload entry")
        name = entry.get("name")
        if not isinstance(name, str) or not name:
            raise ReleasePayloadError("release payload entry has invalid name")
        if name in by_name:
            raise ReleasePayloadError(f"duplicate release payload: {name}")
        by_name[name] = entry

    if set(by_name) != set(payloads):
        raise ReleasePayloadError(
            f"release payload set mismatch: manifest={sorted(by_name)} supplied={sorted(payloads)}"
        )
    seen_paths: set[Path] = set()
    for name, path in payloads.items():
        if not path.is_file():
            raise ReleasePayloadError(f"{name}: file not found: {path}")
        resolved = path.resolve()
        if resolved in seen_paths:
            raise ReleasePayloadError(f"duplicate supplied payload path: {path}")
        seen_paths.add(resolved)
        entry = by_name[name]
        digest = sha256_file(path)
        if entry.get("sha256") != digest:
            raise ReleasePayloadError(
                f"{name}: payload digest mismatch: expected {digest}, got {entry.get('sha256')}"
            )
        if entry.get("size_bytes") != path.stat().st_size:
            raise ReleasePayloadError(f"{name}: payload size mismatch")
    return value


__all__ = [
    "SCHEMA",
    "VERSION",
    "ReleasePayloadError",
    "create_release_payload_manifest",
    "validate_release_payload_manifest",
]
