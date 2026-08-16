"""Independent validator for compiler-emitted structural proof type registries."""
from __future__ import annotations

from typing import Any

SCHEMA = "arukellt-proof-type-registry"
VERSION = 1
IDENTITY = "TypeInfo-tag-type-args-v1"
MEMORY_MODEL = "arukellt-readonly-heap-v1"
SCALAR_TYPE_IDS = {1, 2, 4}


def validate_document(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError("$: expected object")
    expected = {"schema", "schema_version", "identity", "pointer_width", "memory_model", "references"}
    if set(value) != expected:
        raise ValueError("$: proof type registry field set mismatch")
    if value["schema"] != SCHEMA or value["schema_version"] != VERSION:
        raise ValueError("$: proof type registry schema identity mismatch")
    if value["identity"] != IDENTITY:
        raise ValueError("$.identity: structural TypeInfo identity mismatch")
    if value["pointer_width"] not in {32, 64}:
        raise ValueError("$.pointer_width: expected 32 or 64")
    if value["memory_model"] != MEMORY_MODEL:
        raise ValueError("$.memory_model: unsupported memory model")
    references = value["references"]
    if not isinstance(references, list):
        raise ValueError("$.references: expected array")
    seen: set[int] = set()
    for index, entry in enumerate(references):
        path = f"$.references[{index}]"
        if not isinstance(entry, dict) or set(entry) != {
            "type_id", "kind", "element_type_id", "length_type_id", "nullable"
        }:
            raise ValueError(f"{path}: invalid reference descriptor fields")
        type_id = entry["type_id"]
        element_type_id = entry["element_type_id"]
        if type(type_id) is not int or type_id in seen:
            raise ValueError(f"{path}.type_id: expected unique integer")
        seen.add(type_id)
        if entry["kind"] != "array":
            raise ValueError(f"{path}.kind: only exact Vec array descriptors are supported")
        if type(element_type_id) is not int or element_type_id not in SCALAR_TYPE_IDS:
            raise ValueError(f"{path}.element_type_id: unsupported scalar element TypeId")
        if type_id != 100 + element_type_id:
            raise ValueError(f"{path}.type_id: structural Vec TypeId mismatch")
        if entry["length_type_id"] != 1:
            raise ValueError(f"{path}.length_type_id: Vec length must use i32 TypeId 1")
        if entry["nullable"] is not True:
            raise ValueError(f"{path}.nullable: compiler Vec proof references must be explicit nullable")
    return value


__all__ = ["IDENTITY", "MEMORY_MODEL", "SCALAR_TYPE_IDS", "SCHEMA", "VERSION", "validate_document"]
