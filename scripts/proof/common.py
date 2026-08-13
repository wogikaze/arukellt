"""Shared primitives for fail-closed formal artifact validation."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn


@dataclass(frozen=True)
class ValidationError(ValueError):
    path: str
    message: str

    def __str__(self) -> str:
        return f"{self.path}: {self.message}"


def fail(path: str, message: str) -> NoReturn:
    raise ValidationError(path, message)


def object_value(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail(path, "expected object")
    return value


def array_value(value: Any, path: str) -> list[Any]:
    if not isinstance(value, list):
        fail(path, "expected array")
    return value


def string_value(value: Any, path: str, *, nonempty: bool = True) -> str:
    if not isinstance(value, str):
        fail(path, "expected string")
    if nonempty and not value:
        fail(path, "must not be empty")
    return value


def int_value(value: Any, path: str, *, minimum: int | None = None) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        fail(path, "expected integer")
    if minimum is not None and value < minimum:
        fail(path, f"must be >= {minimum}")
    return value


def bool_value(value: Any, path: str) -> bool:
    if not isinstance(value, bool):
        fail(path, "expected boolean")
    return value


def exact_keys(obj: dict[str, Any], path: str, *, required: set[str], optional: set[str] = set()) -> None:
    missing = required - obj.keys()
    if missing:
        fail(path, f"missing field(s): {', '.join(sorted(missing))}")
    unknown = obj.keys() - required - optional
    if unknown:
        fail(path, f"unknown field(s): {', '.join(sorted(unknown))}")


def sha256_value(value: Any, path: str) -> str:
    digest = string_value(value, path)
    if len(digest) != 64 or any(ch not in "0123456789abcdef" for ch in digest):
        fail(path, "expected lowercase SHA-256 hex")
    return digest


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json_sha256(value: Any) -> str:
    encoded = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def json_file_sha256(path: Path) -> str:
    return canonical_json_sha256(load_json(path))


def validate_header(obj: dict[str, Any], path: str, schema: str, version: int) -> None:
    if string_value(obj["schema"], f"{path}.schema") != schema:
        fail(f"{path}.schema", f"expected {schema!r}")
    if int_value(obj["schema_version"], f"{path}.schema_version") != version:
        fail(f"{path}.schema_version", f"expected {version}")


def unique_id_map(items: list[Any], path: str) -> dict[int, dict[str, Any]]:
    result: dict[int, dict[str, Any]] = {}
    for index, raw in enumerate(items):
        item_path = f"{path}[{index}]"
        item = object_value(raw, item_path)
        item_id = int_value(item.get("id"), f"{item_path}.id", minimum=0)
        if item_id in result:
            fail(f"{item_path}.id", f"duplicate id: {item_id}")
        result[item_id] = item
    return result
