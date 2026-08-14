"""Phase 5 pure aggregate type validation and deterministic identities."""
from __future__ import annotations
from typing import Any

AGGREGATE_KINDS = {"tuple", "struct", "enum"}

def is_aggregate_type(entry: dict[str, Any]) -> bool:
    return entry.get("kind") in AGGREGATE_KINDS

def type_dependencies(entry: dict[str, Any]) -> list[int]:
    kind = entry.get("kind")
    if kind == "tuple": return [int(value) for value in entry.get("elements", [])]
    if kind == "struct": return [int(field["type_id"]) for field in entry.get("fields", [])]
    if kind == "enum": return [int(value) for variant in entry.get("variants", []) for value in variant.get("payload_type_ids", [])]
    return []

def validate_aggregate_types(document: dict[str, Any]) -> None:
    types = {int(entry["id"]): entry for entry in document["types"]}
    for type_id, entry in types.items():
        kind = entry.get("kind")
        if kind not in AGGREGATE_KINDS: continue
        representation = entry.get("representation")
        if not isinstance(representation, dict) or representation.get("nullable") is not False:
            raise ValueError(f"$.types[id={type_id}].representation: pure aggregate must be non-null")
        if kind == "tuple":
            elements = entry.get("elements")
            if not isinstance(elements, list) or not elements: raise ValueError(f"$.types[id={type_id}].elements: expected non-empty array")
        elif kind == "struct":
            fields = entry.get("fields")
            if not isinstance(fields, list) or not fields: raise ValueError(f"$.types[id={type_id}].fields: expected non-empty array")
            names: set[str] = set()
            for index, field in enumerate(fields):
                if not isinstance(field, dict) or set(field) != {"name", "type_id"}: raise ValueError(f"$.types[id={type_id}].fields[{index}]: expected name/type_id")
                name = field["name"]
                if not isinstance(name, str) or not name or name in names: raise ValueError(f"$.types[id={type_id}].fields[{index}].name: field names must be unique")
                names.add(name)
        else:
            variants = entry.get("variants")
            if not isinstance(variants, list) or not variants: raise ValueError(f"$.types[id={type_id}].variants: expected non-empty array")
            names: set[str] = set(); discriminants: set[int] = set()
            for index, variant in enumerate(variants):
                if not isinstance(variant, dict) or set(variant) != {"name", "discriminant", "payload_type_ids"}: raise ValueError(f"$.types[id={type_id}].variants[{index}]: expected name/discriminant/payload_type_ids")
                name = variant["name"]; discriminant = variant["discriminant"]
                if not isinstance(name, str) or not name or name in names: raise ValueError(f"$.types[id={type_id}].variants[{index}].name: variant names must be unique")
                if type(discriminant) is not int or discriminant < 0 or discriminant in discriminants: raise ValueError(f"$.types[id={type_id}].variants[{index}].discriminant: discriminants must be unique non-negative integers")
                if not isinstance(variant["payload_type_ids"], list): raise ValueError(f"$.types[id={type_id}].variants[{index}].payload_type_ids: expected array")
                names.add(name); discriminants.add(discriminant)
        for dependency in type_dependencies(entry):
            if dependency not in types: raise ValueError(f"$.types[id={type_id}]: unknown aggregate member TypeId {dependency}")
    visiting: set[int] = set(); complete: set[int] = set()
    def visit(type_id: int) -> None:
        if type_id in complete: return
        if type_id in visiting: raise ValueError(f"$.types[id={type_id}]: recursive aggregate types are outside proof phase 5")
        visiting.add(type_id)
        for dependency in type_dependencies(types[type_id]):
            if is_aggregate_type(types[dependency]): visit(dependency)
        visiting.remove(type_id); complete.add(type_id)
    for type_id, entry in types.items():
        if is_aggregate_type(entry): visit(type_id)

def aggregate_order(document: dict[str, Any]) -> list[int]:
    types = {int(entry["id"]): entry for entry in document["types"]}; result: list[int] = []; seen: set[int] = set()
    def visit(type_id: int) -> None:
        if type_id in seen: return
        for dependency in type_dependencies(types[type_id]):
            if is_aggregate_type(types[dependency]): visit(dependency)
        seen.add(type_id); result.append(type_id)
    for type_id in sorted(types):
        if is_aggregate_type(types[type_id]): visit(type_id)
    return result

def smt_sort(type_id: int) -> str: return f"A{type_id}"
def smt_constructor(type_id: int, index: int = 0) -> str: return f"a{type_id}_c{index}"
def smt_selector(type_id: int, index: int, payload_index: int | None = None) -> str: return f"a{type_id}_s{index}" if payload_index is None else f"a{type_id}_v{index}_p{payload_index}"

__all__ = ["AGGREGATE_KINDS", "aggregate_order", "is_aggregate_type", "smt_constructor", "smt_selector", "smt_sort", "type_dependencies", "validate_aggregate_types"]
