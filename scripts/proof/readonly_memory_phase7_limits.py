"""Explicit composition limits for the initial Phase 7 memory profile."""
from __future__ import annotations

from typing import Any


def validate_phase7_reference_composition(document: dict[str, Any]) -> None:
    types = {int(entry["id"]): entry for entry in document.get("types", [])}
    reference_ids = {type_id for type_id, entry in types.items() if entry.get("kind") == "reference"}
    for type_id, entry in types.items():
        kind = entry.get("kind")
        members: list[int] = []
        if kind == "tuple":
            members = [int(value) for value in entry.get("elements", [])]
        elif kind == "struct":
            members = [int(field["type_id"]) for field in entry.get("fields", [])]
        elif kind == "enum":
            members = [int(value) for variant in entry.get("variants", []) for value in variant.get("payload_type_ids", [])]
        if any(member in reference_ids for member in members):
            raise ValueError(f"$.types[id={type_id}]: aggregate-contained references are outside initial Phase 7")


__all__ = ["validate_phase7_reference_composition"]
