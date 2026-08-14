"""TypedCoreHIR v1 -> VerifiedCore v1 lowering through Phase 4."""
from __future__ import annotations
import copy
from typing import Any
from proof import typed_corehir_program_convert as legacy
from proof.verified_core_typed import validate_typed_document

SOURCE_SCHEMA = legacy.SOURCE_SCHEMA
SOURCE_VERSION = legacy.SOURCE_VERSION
CONVERTER = "arukellt-typed-corehir-converter-v4"
ExplicitTypedCoreHirError = legacy.ExplicitTypedCoreHirError


class Phase4Lowerer(legacy._Lowerer):
    def __init__(self, function, expressions, contract_ids, callee_ids, used_types, path, next_proof_id):
        super().__init__(function, expressions, contract_ids, callee_ids, used_types, path)
        self.next_proof_id = next_proof_id

    def _proof(self, expression_id: int, path: str) -> dict[str, Any]:
        return legacy._proof_expression(
            expression_id,
            self.expressions,
            self.local_ids,
            "",
            self.next_proof_id,
            self.used_types,
            set(),
            path,
        )

    def eval(self, expression_id: int, block_id: int, stack: set[int] | None = None):
        expression = self.expressions.get(expression_id)
        if not isinstance(expression, dict) or expression.get("kind") != "while":
            return super().eval(expression_id, block_id, stack)
        stack = set() if stack is None else stack
        legacy._require(expression_id not in stack, f"{self.path}.body: expression cycle at {expression_id}")
        stack = {*stack, expression_id}
        expr_path = f"{self.path}.body.expression[id={expression_id}]"
        children = [int(child) for child in expression.get("children", [])]
        legacy._require(int(expression["type_id"]) == 0, f"{expr_path}: while must have unit type")
        legacy._require(len(children) >= 4, f"{expr_path}: proof while requires invariant and decreases annotations")
        invariant_ids = children[2:-1]
        legacy._require(invariant_ids, f"{expr_path}: proof while requires at least one invariant")

        header = self.new_block()
        self.set_terminator(block_id, {"kind": "goto", "target": header, "arguments": []})
        self.blocks[header]["loop"] = {
            "invariants": [self._proof(child, f"{expr_path}.invariants[{index}]") for index, child in enumerate(invariant_ids)],
            "decreases": self._proof(children[-1], f"{expr_path}.decreases"),
        }

        condition_end, condition, terminated = self.eval(children[0], header, stack)
        legacy._require(not terminated and condition is not None, f"{expr_path}: loop condition missing")
        legacy._require(condition_end == header, f"{expr_path}: control-flow loop conditions are outside phase 4")
        body_block = self.new_block()
        exit_block = self.new_block()
        self.set_terminator(header, {
            "kind": "branch",
            "condition": condition,
            "then_target": body_block,
            "else_target": exit_block,
            "then_arguments": [],
            "else_arguments": [],
        })
        body_end, _, body_terminated = self.eval(children[1], body_block, stack)
        if not body_terminated:
            self.set_terminator(body_end, {"kind": "goto", "target": header, "arguments": []})
        return exit_block, None, False


def convert_document(value: Any) -> dict[str, Any]:
    source = legacy.validate_typed_corehir(value)
    legacy._require(source.get("schema") == SOURCE_SCHEMA and source.get("schema_version") == SOURCE_VERSION, "$: unsupported TypedCoreHIR schema")
    source_types = {int(entry["id"]): entry for entry in source["types"]}
    legacy._require(0 in source_types and source_types[0].get("kind") == "unit", "$.types: type id 0 must be unit")
    contracted = [function for function in source["functions"] if function.get("contracts")]
    legacy._require(contracted, "$.functions: no contracted functions")
    callee_ids = legacy._callee_map(contracted)
    used_types: set[int] = {0}
    verified_functions: list[dict[str, Any]] = []

    for function_index, function in enumerate(contracted):
        path = f"$.functions[contracted={function_index}]"
        signature = copy.deepcopy(function["signature"])
        return_type = int(signature["return_type_id"])
        used_types.add(return_type)
        for parameter in signature["parameters"]:
            used_types.add(int(parameter["type_id"]))
        root_id, expressions = legacy._expression_index(function, path)
        locals_rendered = [copy.deepcopy(local) for local in function["locals"]]
        local_ids = {str(local["name"]): int(local["id"]) for local in locals_rendered}
        for local in locals_rendered:
            used_types.add(int(local["type_id"]))
        contract_ids: set[int] = set()
        contracts: list[dict[str, Any]] = []
        next_proof_id = [0]
        for contract_index, contract in enumerate(function["contracts"]):
            contract_path = f"{path}.contracts[{contract_index}]"
            kind = str(contract["kind"])
            legacy._require(kind in {"requires", "ensures"}, f"{contract_path}.kind: function proof contracts require requires/ensures")
            expression_id = int(contract["expression_id"])
            contract_ids.add(expression_id)
            result_name = str(contract.get("result_name", "result" if kind == "ensures" else ""))
            rendered: dict[str, Any] = {
                "kind": kind,
                "expression": legacy._proof_expression(expression_id, expressions, local_ids, result_name, next_proof_id, used_types, set(), f"{contract_path}.expression"),
            }
            if result_name:
                rendered["result_name"] = result_name
            contracts.append(rendered)

        lowerer_source = copy.deepcopy(function)
        lowerer_source["locals"] = locals_rendered
        lowerer = Phase4Lowerer(lowerer_source, expressions, contract_ids, callee_ids, used_types, path, next_proof_id)
        final_locals, body = lowerer.finish(root_id, return_type)
        verified_functions.append({
            "id": int(function["id"]),
            "name": str(function["name"]),
            "signature": signature,
            "abi": copy.deepcopy(function["abi"]),
            "locals": final_locals,
            "contracts": contracts,
            "body": body,
        })

    missing = sorted(type_id for type_id in used_types if type_id not in source_types)
    legacy._require(not missing, f"$.types: missing reachable TypeIds {missing}")
    result = {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": CONVERTER,
        "module": source["module"],
        "target_profile": copy.deepcopy(source["target_profile"]),
        "types": [legacy._verified_type(source_types[type_id], f"$.types[id={type_id}]") for type_id in sorted(used_types)],
        "functions": verified_functions,
    }
    return validate_typed_document(result)


__all__ = ["CONVERTER", "ExplicitTypedCoreHirError", "SOURCE_SCHEMA", "SOURCE_VERSION", "convert_document"]
