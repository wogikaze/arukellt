"""Independent Phase 4 semantic admission for annotated while loops."""
from __future__ import annotations
import copy
from typing import Any
from proof import verified_core_typed_impl as legacy

TypedVerifiedCoreError = legacy.TypedVerifiedCoreError


def has_loops(document: dict[str, Any]) -> bool:
    return any("loop" in block for function in document["functions"] for block in function["body"]["blocks"])


def _successors(block: dict[str, Any]) -> list[int]:
    term = block["terminator"]
    if term["kind"] == "goto":
        return [int(term["target"])]
    if term["kind"] == "branch":
        return [int(term["then_target"]), int(term["else_target"])]
    return []


def _body_region(blocks: dict[int, dict[str, Any]], body_target: int, header: int, exit_target: int, path: str) -> set[int]:
    region: set[int] = set()
    stack = [body_target]
    while stack:
        block_id = stack.pop()
        if block_id == header:
            continue
        if block_id == exit_target:
            raise TypedVerifiedCoreError(f"{path}: break-to-loop-exit is outside phase 4")
        if block_id in region:
            continue
        if block_id not in blocks:
            raise TypedVerifiedCoreError(f"{path}: loop body targets unknown block {block_id}")
        if "loop" in blocks[block_id]:
            raise TypedVerifiedCoreError(f"{path}: nested annotated loops are outside phase 4")
        region.add(block_id)
        term = blocks[block_id]["terminator"]
        if term["kind"] == "branch" and (int(term["then_target"]) == header or int(term["else_target"]) == header):
            raise TypedVerifiedCoreError(f"{path}: branch continue edges are outside phase 4")
        for target in _successors(blocks[block_id]):
            if target != header:
                stack.append(target)
    return region


def validate_loop_semantics(document: dict[str, Any]) -> None:
    kinds = legacy._type_kinds(document)
    for function_index, function in enumerate(document["functions"]):
        path = f"$.functions[{function_index}]"
        blocks = {int(block["id"]): block for block in function["body"]["blocks"]}
        headers = [block for block in function["body"]["blocks"] if "loop" in block]
        if not headers:
            continue
        local_types = {int(local["id"]): int(local["type_id"]) for local in function["locals"]}
        return_type = int(function["signature"]["return_type_id"])
        seen_ids: set[int] = set()
        for header_block in headers:
            header = int(header_block["id"])
            loop_path = f"{path}.body.blocks[id={header}].loop"
            term = header_block["terminator"]
            if term["kind"] != "branch":
                raise TypedVerifiedCoreError(f"{loop_path}: annotated while header must terminate with branch")
            metadata = header_block["loop"]
            for invariant_index, invariant in enumerate(metadata["invariants"]):
                type_id = legacy._expression_type(invariant, path=f"{loop_path}.invariants[{invariant_index}]", kinds=kinds, local_types=local_types, return_type=return_type, allow_result=False, seen_ids=seen_ids)
                if kinds[type_id] != "bool":
                    raise TypedVerifiedCoreError(f"{loop_path}.invariants[{invariant_index}]: invariant must have type bool")
            decreases_type = legacy._expression_type(metadata["decreases"], path=f"{loop_path}.decreases", kinds=kinds, local_types=local_types, return_type=return_type, allow_result=False, seen_ids=seen_ids)
            if kinds[decreases_type] != "integer":
                raise TypedVerifiedCoreError(f"{loop_path}.decreases: decreases must have integer type")
            body_target = int(term["then_target"])
            exit_target = int(term["else_target"])
            region = _body_region(blocks, body_target, header, exit_target, loop_path)
            backedges: list[dict[str, Any]] = []
            for block_id in sorted(region):
                body_term = blocks[block_id]["terminator"]
                if body_term["kind"] == "goto" and int(body_term["target"]) == header:
                    backedges.append(body_term)
            if len(backedges) != 1:
                raise TypedVerifiedCoreError(f"{loop_path}: phase 4 requires exactly one explicit goto backedge")
            expected = [int(parameter["type_id"]) for parameter in header_block["parameters"]]
            actual = [legacy._value_type(value, local_types, kinds, f"{loop_path}.backedge.arguments[{index}]") for index, value in enumerate(backedges[0]["arguments"])]
            if actual != expected:
                raise TypedVerifiedCoreError(f"{loop_path}: backedge block parameter mismatch")


def legacy_validation_view(document: dict[str, Any]) -> dict[str, Any]:
    """Return an acyclic copy used only to reuse the Phase 0-3 type checker."""
    value = copy.deepcopy(document)
    kinds = {int(entry["id"]): str(entry["kind"]) for entry in value["types"]}
    for function in value["functions"]:
        headers = {int(block["id"]) for block in function["body"]["blocks"] if "loop" in block}
        return_type = int(function["signature"]["return_type_id"])
        return_kind = kinds[return_type]
        for block in function["body"]["blocks"]:
            block.pop("loop", None)
            term = block["terminator"]
            if term["kind"] != "goto" or int(term["target"]) not in headers:
                continue
            if return_kind == "unit":
                block["terminator"] = {"kind": "return"}
            elif return_kind == "integer":
                block["terminator"] = {"kind": "return", "value": {"kind": "constant", "type_id": return_type, "value": 0}}
            elif return_kind == "bool":
                block["terminator"] = {"kind": "return", "value": {"kind": "constant", "type_id": return_type, "value": False}}
            else:
                raise TypedVerifiedCoreError("phase 4 validation view supports scalar returns only")
    return value


__all__ = ["TypedVerifiedCoreError", "has_loops", "legacy_validation_view", "validate_loop_semantics"]
