"""Validation-only compatibility view for Phase 7 reference artifacts."""
from __future__ import annotations

from typing import Any

from proof.readonly_memory_semantics import compatibility_view as _base_compatibility_view


def compatibility_view(document: dict[str, Any]) -> dict[str, Any]:
    value = _base_compatibility_view(document)
    reference_ids = {int(entry["id"]) for entry in value.get("types", []) if entry.get("kind") == "reference"}
    for entry in value.get("types", []):
        if int(entry.get("id", -1)) not in reference_ids:
            continue
        representation = entry.get("representation", {})
        if representation.get("wasm") == ["gc-ref"]:
            representation["wasm"] = ["externref"]
    for function in value.get("functions", []):
        abi = function.get("abi", {})
        for item in list(abi.get("parameters", [])) + list(abi.get("results", [])):
            if int(item.get("type_id", -1)) in reference_ids and item.get("wasm") == ["gc-ref"]:
                item["wasm"] = ["externref"]
    return value


__all__ = ["compatibility_view"]
