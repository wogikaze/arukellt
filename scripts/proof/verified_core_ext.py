"""Structural admission extensions for VerifiedCore v1 proof phases."""
from __future__ import annotations
import copy
from typing import Any
from proof import verified_core_program as legacy
from proof.aggregate_compat_v5 import compatibility_view as aggregate_compatibility_view
from proof.aggregate_semantics import has_aggregates
from proof.verified_core_interface import validate_call_interface_binding
SCHEMA = legacy.SCHEMA
VERSION = legacy.VERSION


def _loop_metadata(loop: Any, path: str, type_ids: set[int], local_types: dict[int, int]) -> None:
    if not isinstance(loop, dict) or set(loop) != {"invariants", "decreases"}:
        raise ValueError(f"{path}: expected invariants/decreases object")
    invariants = loop["invariants"]
    if not isinstance(invariants, list) or not invariants:
        raise ValueError(f"{path}.invariants: expected non-empty array")
    for index, expression in enumerate(invariants):
        legacy._expression(expression, f"{path}.invariants[{index}]", type_ids, local_types)
    legacy._expression(loop["decreases"], f"{path}.decreases", type_ids, local_types)


def _abi_signature(function: dict[str, Any], path: str) -> None:
    signature = function["signature"]
    abi = function["abi"]
    parameter_types = [int(parameter["type_id"]) for parameter in signature["parameters"]]
    abi_parameters = abi["parameters"]
    if len(abi_parameters) != len(parameter_types):
        raise ValueError(f"{path}.abi.parameters: ABI parameter count must match signature")
    for index, expected in enumerate(parameter_types):
        item = abi_parameters[index]
        if not isinstance(item, dict) or type(item.get("type_id")) is not int:
            raise ValueError(f"{path}.abi.parameters[{index}]: invalid ABI item")
        if int(item["type_id"]) != expected:
            raise ValueError(
                f"{path}.abi.parameters[{index}].type_id: ABI type must match signature"
            )

    return_type = int(signature["return_type_id"])
    abi_results = abi["results"]
    expected_count = 0 if return_type == 0 else 1
    if len(abi_results) != expected_count:
        raise ValueError(f"{path}.abi.results: ABI result count must match signature")
    if abi_results:
        item = abi_results[0]
        if not isinstance(item, dict) or type(item.get("type_id")) is not int:
            raise ValueError(f"{path}.abi.results[0]: invalid ABI item")
        if int(item["type_id"]) != return_type:
            raise ValueError(
                f"{path}.abi.results[0].type_id: ABI type must match signature"
            )


def validate_document(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        return legacy.validate_document(value)
    document = copy.deepcopy(value)
    compatibility = copy.deepcopy(document)
    # Phase 5 aggregate selectors/constructors carry fields unknown to the
    # frozen v1 structural validator. They are independently type-checked by
    # aggregate_semantics; only the validation copy is projected to v1 here.
    if has_aggregates(document):
        compatibility = aggregate_compatibility_view(compatibility)
    for function in compatibility.get("functions", []):
        for block in function.get("body", {}).get("blocks", []):
            block.pop("loop", None)
            for instruction in block.get("instructions", []):
                if instruction.get("op") == "call":
                    instruction.pop("callee_interface_sha256", None)
    legacy.validate_document(compatibility)
    type_ids = {int(entry["id"]) for entry in document["types"]}
    functions = {int(function["id"]): function for function in document["functions"]}
    for fi, function in enumerate(document["functions"]):
        function_path = f"$.functions[{fi}]"
        _abi_signature(function, function_path)
        local_types = {int(local["id"]): int(local["type_id"]) for local in function["locals"]}
        for bi, block in enumerate(function["body"]["blocks"]):
            block_path = f"{function_path}.body.blocks[{bi}]"
            if "loop" in block:
                _loop_metadata(block["loop"], f"{block_path}.loop", type_ids, local_types)
            for ii, instruction in enumerate(block["instructions"]):
                if instruction.get("op") != "call":
                    continue
                path = f"{block_path}.instructions[{ii}]"
                callee_id = int(instruction["callee_id"])
                if callee_id not in functions:
                    raise ValueError(f"{path}.callee_id: unknown function")
                validate_call_interface_binding(instruction, functions[callee_id], path)
    return document

__all__ = ["SCHEMA", "VERSION", "validate_document"]
