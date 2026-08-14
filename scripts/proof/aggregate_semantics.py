"""Phase 5 pure aggregate semantic admission."""
from __future__ import annotations
from typing import Any
from proof import verified_core_typed_impl as legacy
from proof.aggregate_types import is_aggregate_type, validate_aggregate_types

AGGREGATE_EXPRESSION_KINDS = {"construct", "project", "is_variant", "variant_payload"}
AGGREGATE_INSTRUCTION_OPS = {"construct", "project", "is_variant", "variant_payload"}


def _types(document: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {int(entry["id"]): entry for entry in document["types"]}


def _bool_type(types: dict[int, dict[str, Any]]) -> int:
    matches = [type_id for type_id, entry in types.items() if entry["kind"] == "bool"]
    if len(matches) != 1:
        raise ValueError("$.types: exactly one bool type is required")
    return matches[0]


def _member_types(entry: dict[str, Any], variant_index: int | None = None) -> list[int]:
    if entry["kind"] == "tuple":
        return [int(value) for value in entry["elements"]]
    if entry["kind"] == "struct":
        return [int(field["type_id"]) for field in entry["fields"]]
    if entry["kind"] == "enum":
        if type(variant_index) is not int or variant_index < 0 or variant_index >= len(entry["variants"]):
            raise ValueError("enum variant index is out of range")
        return [int(value) for value in entry["variants"][variant_index]["payload_type_ids"]]
    raise ValueError("aggregate type required")


def _value_type(value: dict[str, Any], local_types: dict[int, int], types: dict[int, dict[str, Any]], path: str) -> int:
    type_id = int(value["type_id"])
    if type_id not in types:
        raise ValueError(f"{path}.type_id: unknown type")
    if value["kind"] == "local":
        if local_types.get(int(value["local_id"])) != type_id:
            raise ValueError(f"{path}.local_id: local type mismatch")
    elif value["kind"] == "constant":
        if is_aggregate_type(types[type_id]):
            raise ValueError(f"{path}: aggregate constants require construct")
        legacy._check_constant(value["value"], type_id, {tid: str(t["kind"]) for tid, t in types.items()}, f"{path}.value")
    else:
        raise ValueError(f"{path}.kind: unsupported value kind")
    return type_id


def has_aggregates(document: dict[str, Any]) -> bool:
    return any(is_aggregate_type(entry) for entry in document.get("types", []))


__all__ = ["AGGREGATE_EXPRESSION_KINDS", "AGGREGATE_INSTRUCTION_OPS", "has_aggregates"]
