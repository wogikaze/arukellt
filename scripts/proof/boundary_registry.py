"""Independent validation for the pinned major-boundary registry.

The registry is intentionally able to describe an unmerged PR stack. Every file
reference is pinned to a full immutable commit SHA; branch names are never part
of the trust decision. Validation may use a fake source in unit tests or fetch
raw files from GitHub for an end-to-end registry check.
"""

from __future__ import annotations

import hashlib
import json
import re
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping, Protocol

SCHEMA = "arukellt-boundary-registry"
VERSION = 1
REQUIRED_BOUNDARIES = {
    "typed-corehir",
    "corehir-body",
    "verified-core",
    "mir-optimizer",
    "backend-layout",
    "solver-result",
    "release-authorization",
    "proof-semantics",
}
ARTIFACT_KINDS = {
    "serialized-json",
    "in-memory-snapshot",
    "typed-mir-snapshot",
}
_SHA1 = re.compile(r"^[0-9a-f]{40}$")
_REPOSITORY = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")


class BoundaryRegistryError(ValueError):
    """The registry is malformed, incomplete, unpinned, or stale."""


class BoundaryFileSource(Protocol):
    def fetch(self, repository: str, commit: str, path: str) -> bytes:
        """Return exact file bytes for a repository path at a commit."""


@dataclass(frozen=True)
class GitHubRawSource:
    timeout_seconds: float = 20.0
    user_agent: str = "arukellt-boundary-registry-v1"

    def fetch(self, repository: str, commit: str, path: str) -> bytes:
        owner, name = repository.split("/", 1)
        encoded_path = "/".join(urllib.parse.quote(part, safe="") for part in path.split("/"))
        url = (
            "https://raw.githubusercontent.com/"
            f"{urllib.parse.quote(owner, safe='')}/"
            f"{urllib.parse.quote(name, safe='')}/{commit}/{encoded_path}"
        )
        request = urllib.request.Request(url, headers={"User-Agent": self.user_agent})
        try:
            with urllib.request.urlopen(request, timeout=self.timeout_seconds) as response:
                data = response.read()
        except (urllib.error.HTTPError, urllib.error.URLError, TimeoutError) as exc:
            raise BoundaryRegistryError(
                f"unable to fetch {repository}@{commit}:{path}: {exc}"
            ) from exc
        if not data:
            raise BoundaryRegistryError(f"empty boundary file: {repository}@{commit}:{path}")
        return data


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise BoundaryRegistryError(message)


def _object(value: object, path: str) -> dict[str, Any]:
    _require(isinstance(value, dict), f"{path}: expected object")
    return value


def _exact_keys(value: dict[str, Any], expected: set[str], path: str) -> None:
    _require(set(value) == expected, f"{path}: field set mismatch")


def _string(value: object, path: str) -> str:
    _require(isinstance(value, str) and bool(value), f"{path}: expected non-empty string")
    return str(value)


def _positive_int(value: object, path: str) -> int:
    _require(type(value) is int and int(value) >= 1, f"{path}: expected integer >= 1")
    return int(value)


def _commit(value: object, path: str) -> str:
    commit = _string(value, path)
    _require(_SHA1.fullmatch(commit) is not None, f"{path}: expected full lowercase commit SHA")
    return commit


def _relative_path(value: object, path: str) -> str:
    raw = _string(value, path)
    candidate = Path(raw)
    _require(not candidate.is_absolute(), f"{path}: absolute paths are forbidden")
    _require(".." not in candidate.parts, f"{path}: path traversal is forbidden")
    _require(raw == candidate.as_posix(), f"{path}: expected canonical POSIX path")
    return raw


def _tokens(value: object, path: str) -> list[str]:
    _require(isinstance(value, list) and bool(value), f"{path}: expected non-empty array")
    rendered = [_string(token, f"{path}[{index}]") for index, token in enumerate(value)]
    _require(len(set(rendered)) == len(rendered), f"{path}: duplicate token")
    return rendered


def _file_ref(value: object, path: str) -> dict[str, Any]:
    ref = _object(value, path)
    _exact_keys(ref, {"commit", "path", "required_tokens"}, path)
    _commit(ref["commit"], f"{path}.commit")
    _relative_path(ref["path"], f"{path}.path")
    _tokens(ref["required_tokens"], f"{path}.required_tokens")
    return ref


def _artifact(value: object, path: str) -> dict[str, Any]:
    artifact = _object(value, path)
    _exact_keys(artifact, {"schema", "schema_version", "kind"}, path)
    _string(artifact["schema"], f"{path}.schema")
    _positive_int(artifact["schema_version"], f"{path}.schema_version")
    kind = _string(artifact["kind"], f"{path}.kind")
    _require(kind in ARTIFACT_KINDS, f"{path}.kind: unsupported artifact kind {kind!r}")
    return artifact


def validate_registry(value: object) -> dict[str, Any]:
    registry = _object(value, "$")
    _exact_keys(
        registry,
        {
            "schema",
            "schema_version",
            "status",
            "repository",
            "required_boundaries",
            "boundaries",
        },
        "$",
    )
    _require(registry["schema"] == SCHEMA, f"$.schema: expected {SCHEMA!r}")
    _require(registry["schema_version"] == VERSION, f"$.schema_version: expected {VERSION}")
    _require(registry["status"] == "enforced", "$.status: expected enforced")
    repository = _string(registry["repository"], "$.repository")
    _require(_REPOSITORY.fullmatch(repository) is not None, "$.repository: expected owner/name")

    required_raw = registry["required_boundaries"]
    _require(isinstance(required_raw, list), "$.required_boundaries: expected array")
    required = [_string(item, f"$.required_boundaries[{index}]") for index, item in enumerate(required_raw)]
    _require(len(required) == len(set(required)), "$.required_boundaries: duplicate boundary")
    _require(set(required) == REQUIRED_BOUNDARIES, "$.required_boundaries: required set mismatch")

    boundaries_raw = registry["boundaries"]
    _require(isinstance(boundaries_raw, list), "$.boundaries: expected array")
    _require(len(boundaries_raw) == len(REQUIRED_BOUNDARIES), "$.boundaries: boundary count mismatch")

    ids: set[str] = set()
    schemas: set[tuple[str, int]] = set()
    for index, raw in enumerate(boundaries_raw):
        path = f"$.boundaries[{index}]"
        boundary = _object(raw, path)
        _exact_keys(
            boundary,
            {
                "id",
                "artifact",
                "producer",
                "validator",
                "consumers",
                "evidence",
                "workflow",
                "failure_action",
            },
            path,
        )
        boundary_id = _string(boundary["id"], f"{path}.id")
        _require(boundary_id in REQUIRED_BOUNDARIES, f"{path}.id: unknown boundary {boundary_id!r}")
        _require(boundary_id not in ids, f"{path}.id: duplicate boundary {boundary_id!r}")
        ids.add(boundary_id)

        artifact = _artifact(boundary["artifact"], f"{path}.artifact")
        identity = (str(artifact["schema"]), int(artifact["schema_version"]))
        _require(identity not in schemas, f"{path}.artifact: duplicate schema identity {identity}")
        schemas.add(identity)

        producer = _file_ref(boundary["producer"], f"{path}.producer")
        validator = _file_ref(boundary["validator"], f"{path}.validator")
        _require(
            (producer["commit"], producer["path"]) != (validator["commit"], validator["path"]),
            f"{path}: producer and validator must be independent files",
        )

        consumers = boundary["consumers"]
        _require(isinstance(consumers, list) and bool(consumers), f"{path}.consumers: expected non-empty array")
        consumer_refs: set[tuple[str, str]] = set()
        for consumer_index, consumer_raw in enumerate(consumers):
            consumer = _file_ref(consumer_raw, f"{path}.consumers[{consumer_index}]")
            key = (str(consumer["commit"]), str(consumer["path"]))
            _require(key not in consumer_refs, f"{path}.consumers: duplicate consumer reference")
            consumer_refs.add(key)

        _file_ref(boundary["evidence"], f"{path}.evidence")
        _file_ref(boundary["workflow"], f"{path}.workflow")
        _string(boundary["failure_action"], f"{path}.failure_action")

    _require(ids == REQUIRED_BOUNDARIES, "$.boundaries: incomplete major-boundary coverage")
    return registry


def iter_file_references(registry: Mapping[str, Any]) -> Iterable[tuple[str, Mapping[str, Any]]]:
    for boundary in registry["boundaries"]:
        boundary_id = boundary["id"]
        yield f"{boundary_id}.producer", boundary["producer"]
        yield f"{boundary_id}.validator", boundary["validator"]
        for index, consumer in enumerate(boundary["consumers"]):
            yield f"{boundary_id}.consumer[{index}]", consumer
        yield f"{boundary_id}.evidence", boundary["evidence"]
        yield f"{boundary_id}.workflow", boundary["workflow"]


def validate_registry_files(
    registry: Mapping[str, Any],
    source: BoundaryFileSource,
) -> dict[tuple[str, str], bytes]:
    validated = validate_registry(dict(registry))
    repository = str(validated["repository"])
    fetched: dict[tuple[str, str], bytes] = {}
    for label, ref in iter_file_references(validated):
        key = (str(ref["commit"]), str(ref["path"]))
        data = fetched.get(key)
        if data is None:
            data = source.fetch(repository, key[0], key[1])
            _require(bool(data), f"{label}: referenced file is empty")
            fetched[key] = data
        try:
            text = data.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise BoundaryRegistryError(f"{label}: referenced file is not UTF-8") from exc
        for token in ref["required_tokens"]:
            _require(token in text, f"{label}: required token missing: {token!r}")
    return fetched


def load_registry(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise BoundaryRegistryError(f"invalid registry JSON: {exc}") from exc
    return validate_registry(value)


__all__ = [
    "ARTIFACT_KINDS",
    "BoundaryFileSource",
    "BoundaryRegistryError",
    "GitHubRawSource",
    "REQUIRED_BOUNDARIES",
    "SCHEMA",
    "VERSION",
    "iter_file_references",
    "load_registry",
    "sha256_bytes",
    "sha256_file",
    "validate_registry",
    "validate_registry_files",
]
