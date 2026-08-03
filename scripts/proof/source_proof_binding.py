"""Digest binding for the source-contract proof pipeline."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Mapping

SCHEMA = "arukellt-source-proof-binding"
VERSION = 1
REQUIRED_ARTIFACTS = (
    "source",
    "producer_executable",
    "typed_corehir",
    "verified_core_machine",
    "verified_core_normalized",
    "solver_input",
)


class SourceProofBindingError(ValueError):
    """A source proof binding is missing data or contains stale digests."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def create_binding(paths: Mapping[str, Path]) -> dict[str, object]:
    missing = [name for name in REQUIRED_ARTIFACTS if name not in paths]
    if missing:
        raise SourceProofBindingError(f"missing binding artifact(s): {missing}")
    artifacts: list[dict[str, str]] = []
    for name in REQUIRED_ARTIFACTS:
        path = paths[name]
        if not path.is_file():
            raise SourceProofBindingError(f"{name}: file not found: {path}")
        artifacts.append(
            {
                "name": name,
                "path": path.as_posix(),
                "sha256": sha256_file(path),
            }
        )
    return {
        "schema": SCHEMA,
        "schema_version": VERSION,
        "artifacts": artifacts,
    }


def write_binding(paths: Mapping[str, Path], output: Path) -> dict[str, object]:
    document = create_binding(paths)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return document


def validate_binding(value: object, paths: Mapping[str, Path]) -> dict[str, object]:
    if not isinstance(value, dict):
        raise SourceProofBindingError("binding must be an object")
    if value.get("schema") != SCHEMA or value.get("schema_version") != VERSION:
        raise SourceProofBindingError("binding schema identity mismatch")
    entries = value.get("artifacts")
    if not isinstance(entries, list):
        raise SourceProofBindingError("binding artifacts must be an array")
    by_name: dict[str, dict[str, object]] = {}
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("name"), str):
            raise SourceProofBindingError("invalid binding artifact entry")
        name = str(entry["name"])
        if name in by_name:
            raise SourceProofBindingError(f"duplicate binding artifact: {name}")
        by_name[name] = entry
    if set(by_name) != set(REQUIRED_ARTIFACTS):
        raise SourceProofBindingError(
            f"binding artifact set mismatch: {sorted(by_name)}"
        )
    for name in REQUIRED_ARTIFACTS:
        path = paths.get(name)
        if path is None or not path.is_file():
            raise SourceProofBindingError(f"{name}: file not found")
        expected = sha256_file(path)
        actual = by_name[name].get("sha256")
        if actual != expected:
            raise SourceProofBindingError(
                f"{name}: digest mismatch: expected {expected}, got {actual}"
            )
    return value


__all__ = [
    "REQUIRED_ARTIFACTS",
    "SCHEMA",
    "VERSION",
    "SourceProofBindingError",
    "create_binding",
    "sha256_file",
    "validate_binding",
    "write_binding",
]
