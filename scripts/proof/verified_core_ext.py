"""Structural admission extensions for VerifiedCore v1 proof phases."""
from __future__ import annotations
import copy
from typing import Any
from proof import verified_core_program as legacy
from proof.verified_core_interface import validate_call_interface_binding
SCHEMA = legacy.SCHEMA
VERSION = legacy.VERSION

def validate_document(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        return legacy.validate_document(value)
    document = copy.deepcopy(value)
    compatibility = copy.deepcopy(document)
    for function in compatibility.get("functions", []):
        for block in function.get("body", {}).get("blocks", []):
            for instruction in block.get("instructions", []):
                if instruction.get("op") == "call":
                    instruction.pop("callee_interface_sha256", None)
    legacy.validate_document(compatibility)
    functions = {int(function["id"]): function for function in document["functions"]}
    for fi, function in enumerate(document["functions"]):
        for bi, block in enumerate(function["body"]["blocks"]):
            for ii, instruction in enumerate(block["instructions"]):
                if instruction.get("op") != "call":
                    continue
                path = f"$.functions[{fi}].body.blocks[{bi}].instructions[{ii}]"
                callee_id = int(instruction["callee_id"])
                if callee_id not in functions:
                    raise ValueError(f"{path}.callee_id: unknown function")
                validate_call_interface_binding(instruction, functions[callee_id], path)
    return document

__all__ = ["SCHEMA", "VERSION", "validate_document"]
