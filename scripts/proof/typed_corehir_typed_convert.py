"""Explicitly typed TypedCoreHIR v1 to VerifiedCore v1 conversion boundary.

The internal converter is permitted to select the contracted proof subject, but
all logical meaning of reachable integer types is re-established from explicit
``bits`` and ``signed`` metadata. Source type names are identity-only.
"""

from __future__ import annotations

import copy
from typing import Any

from proof.typed_corehir_convert import (
    UnsupportedTypedCoreHir,
    convert_document as convert_legacy_document,
)
from proof.verified_core_typed import validate_typed_document

SOURCE_SCHEMA = "arukellt-typed-corehir"
SOURCE_VERSION = 1
CONVERTER = "arukellt-typed-corehir-converter-v2"


class ExplicitTypedCoreHirError(UnsupportedTypedCoreHir):
    """Reachable proof types lack explicit or consistent logical metadata."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ExplicitTypedCoreHirError(message)


def _require_int(value: object, path: str, *, minimum: int = 0) -> int:
    if type(value) is not int or int(value) < minimum:
        raise ExplicitTypedCoreHirError(f"{path}: expected integer >= {minimum}")
    return int(value)


def _require_bool(value: object, path: str) -> bool:
    if type(value) is not bool:
        raise ExplicitTypedCoreHirError(f"{path}: expected boolean")
    return bool(value)


def _require_string(value: object, path: str) -> str:
    if not isinstance(value, str) or not value:
        raise ExplicitTypedCoreHirError(f"{path}: expected non-empty string")
    return value


def _validate_representation(
    entry: dict[str, Any],
    *,
    path: str,
    expected_value_type: str,
    expected_wasm: list[str],
    expected_size: int,
) -> None:
    value_type = _require_string(entry.get("value_type"), f"{path}.value_type")
    _require(
        value_type == expected_value_type,
        f"{path}.value_type: expected {expected_value_type!r}, got {value_type!r}",
    )
    representation = entry.get("representation")
    _require(isinstance(representation, dict), f"{path}.representation: expected object")
    _require(
        representation.get("wasm") == expected_wasm,
        f"{path}.representation.wasm: explicit type mismatch",
    )
    _require(
        representation.get("nullable") is False,
        f"{path}.representation.nullable: scalar type must be non-nullable",
    )
    size = _require_int(
        representation.get("size_bytes"), f"{path}.representation.size_bytes"
    )
    _require(size == expected_size, f"{path}.representation.size_bytes: explicit type mismatch")
    align = _require_int(
        representation.get("align_bytes"),
        f"{path}.representation.align_bytes",
        minimum=1,
    )
    _require(
        align <= expected_size if expected_size else align == 1,
        f"{path}.representation.align_bytes: invalid scalar alignment",
    )


def _normalize_source(
    source: dict[str, Any],
) -> tuple[dict[str, Any], dict[int, dict[str, Any]]]:
    raw_types = source.get("types")
    _require(isinstance(raw_types, list) and raw_types, "$.types: expected non-empty array")
    normalized = copy.deepcopy(source)
    normalized_types: list[dict[str, Any]] = []
    explicit_by_id: dict[int, dict[str, Any]] = {}

    for index, raw in enumerate(raw_types):
        path = f"$.types[{index}]"
        _require(isinstance(raw, dict), f"{path}: expected object")
        type_id = _require_int(raw.get("id"), f"{path}.id")
        _require(type_id not in explicit_by_id, f"{path}.id: duplicate type id {type_id}")
        kind = _require_string(raw.get("kind"), f"{path}.kind")
        _require_string(raw.get("name"), f"{path}.name")
        explicit = copy.deepcopy(raw)
        legacy = copy.deepcopy(raw)

        if kind == "integer":
            bits = _require_int(raw.get("bits"), f"{path}.bits", minimum=1)
            signed = _require_bool(raw.get("signed"), f"{path}.signed")
            _require(signed and bits in {32, 64}, f"{path}: only explicit signed i32/i64 are supported")
            expected = f"i{bits}"
            _validate_representation(
                raw,
                path=path,
                expected_value_type=expected,
                expected_wasm=[expected],
                expected_size=bits // 8,
            )
            # Canonical internal normalization derives from explicit metadata.
            # The source identity name is restored after conversion.
            legacy["name"] = expected

        explicit_by_id[type_id] = explicit
        normalized_types.append(legacy)

    normalized["types"] = normalized_types
    return normalized, explicit_by_id


def _admit_reachable_type(
    rendered: dict[str, Any], explicit: dict[str, Any], *, path: str
) -> None:
    kind = explicit.get("kind")
    _require(rendered.get("kind") == kind, f"{path}.kind: converter changed type kind")
    if kind == "integer":
        bits = _require_int(explicit.get("bits"), f"{path}.bits", minimum=1)
        signed = _require_bool(explicit.get("signed"), f"{path}.signed")
        _require(signed and bits in {32, 64}, f"{path}: unsupported proof integer")
        expected = f"i{bits}"
        _validate_representation(
            explicit,
            path=path,
            expected_value_type=expected,
            expected_wasm=[expected],
            expected_size=bits // 8,
        )
        rendered["bits"] = bits
        rendered["signed"] = signed
    elif kind == "bool":
        _validate_representation(
            explicit,
            path=path,
            expected_value_type="i32",
            expected_wasm=["i32"],
            expected_size=4,
        )
    elif kind == "unit":
        _validate_representation(
            explicit,
            path=path,
            expected_value_type="void",
            expected_wasm=[],
            expected_size=0,
        )
    else:
        raise ExplicitTypedCoreHirError(
            f"{path}.kind: unsupported reachable proof type {kind!r}"
        )

    rendered["name"] = explicit["name"]
    representation = explicit["representation"]
    _require(
        rendered["representation"] == {
            "wasm": representation["wasm"],
            "nullable": representation["nullable"],
            "size_bytes": representation["size_bytes"],
            "align_bytes": representation["align_bytes"],
        },
        f"{path}.representation: converter changed explicit representation",
    )


def convert_typed_document(value: Any) -> dict[str, Any]:
    _require(isinstance(value, dict), "$: expected object")
    source = copy.deepcopy(value)
    _require(source.get("schema") == SOURCE_SCHEMA, f"$.schema: expected {SOURCE_SCHEMA!r}")
    _require(source.get("schema_version") == SOURCE_VERSION, f"$.schema_version: expected {SOURCE_VERSION}")

    normalized, explicit_types = _normalize_source(source)
    try:
        converted = convert_legacy_document(normalized)
    except UnsupportedTypedCoreHir as exc:
        raise ExplicitTypedCoreHirError(str(exc)) from exc

    converted_by_id = {int(entry["id"]): entry for entry in converted["types"]}
    unknown = sorted(set(converted_by_id) - set(explicit_types))
    _require(not unknown, f"$.types: converter introduced unknown type ids {unknown}")
    for type_id, rendered in converted_by_id.items():
        explicit = explicit_types[type_id]
        _admit_reachable_type(rendered, explicit, path=f"$.types[id={type_id}]")

    converted["generator"] = CONVERTER
    return validate_typed_document(converted)


__all__ = [
    "CONVERTER",
    "ExplicitTypedCoreHirError",
    "SOURCE_SCHEMA",
    "SOURCE_VERSION",
    "convert_typed_document",
]
