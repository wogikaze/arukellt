"""Contract-only TypedCoreHIR v1 to VerifiedCore v1 conversion.

This boundary reuses the fail-closed expression conversion helpers from
``typed_corehir_convert`` while owning proof-subject selection and mandatory
VerifiedCore invariants such as the unit type at ID 0.
"""

from __future__ import annotations

import copy
from typing import Any

from proof import typed_corehir_convert as base
from proof.common import array_value, int_value, object_value, string_value
from proof.typed_corehir import validate_document as validate_typed_corehir
from proof.verified_core import validate_document as validate_verified_core

UnsupportedTypedCoreHir = base.UnsupportedTypedCoreHir


def convert_document(value: Any) -> dict[str, Any]:
    source = validate_typed_corehir(value)
    source_types = base._index_types(source)
    unit_type = source_types.get(0)
    if unit_type is None or string_value(unit_type["kind"], "$.types[id=0].kind") != "unit":
        raise UnsupportedTypedCoreHir("$.types: type id 0 must be unit")

    # VerifiedCore reserves ID 0 for unit even when the proof subject does not
    # explicitly mention unit-valued expressions.
    used_types: set[int] = {0}
    verified_functions: list[dict[str, Any]] = []

    for function_index, raw in enumerate(array_value(source["functions"], "$.functions")):
        path = f"$.functions[{function_index}]"
        function = object_value(raw, path)
        source_contracts = array_value(function["contracts"], f"{path}.contracts")
        if not source_contracts:
            continue

        rendered_locals, local_ids, local_types = base._locals(function, path)
        used_types.update(local_types)
        signature = object_value(function["signature"], f"{path}.signature")
        used_types.add(int_value(signature["return_type_id"], f"{path}.signature.return_type_id", minimum=0))
        for parameter_index, parameter in enumerate(signature["parameters"]):
            used_types.add(
                int_value(
                    parameter["type_id"],
                    f"{path}.signature.parameters[{parameter_index}].type_id",
                    minimum=0,
                )
            )

        root_id, expressions = base._expression_index(function, path)
        contracts: list[dict[str, Any]] = []
        contract_ids: set[int] = set()
        next_proof_id = [0]
        for contract_index, raw_contract in enumerate(source_contracts):
            contract_path = f"{path}.contracts[{contract_index}]"
            contract = object_value(raw_contract, contract_path)
            kind = string_value(contract["kind"], f"{contract_path}.kind")
            if kind not in {"requires", "ensures"}:
                raise UnsupportedTypedCoreHir(
                    f"{contract_path}.kind: unsupported contract {kind!r}"
                )
            expression_id = int_value(
                contract["expression_id"],
                f"{contract_path}.expression_id",
                minimum=0,
            )
            contract_ids.add(expression_id)
            result_name = str(
                contract.get("result_name", "result" if kind == "ensures" else "")
            )
            rendered: dict[str, Any] = {
                "kind": kind,
                "expression": base._proof_expression(
                    expression_id,
                    expressions,
                    local_ids,
                    result_name,
                    set(),
                    next_proof_id,
                    used_types,
                    f"{contract_path}.expression",
                ),
            }
            if result_name:
                rendered["result_name"] = result_name
            contracts.append(rendered)

        return_value = base._return_value(
            root_id,
            expressions,
            contract_ids,
            local_ids,
            used_types,
            path,
        )
        verified_functions.append(
            {
                "id": function["id"],
                "name": function["name"],
                "signature": copy.deepcopy(signature),
                "abi": copy.deepcopy(function["abi"]),
                "locals": rendered_locals,
                "contracts": contracts,
                "body": {
                    "entry_block": 0,
                    "blocks": [
                        {
                            "id": 0,
                            "parameters": [],
                            "instructions": [],
                            "terminator": {"kind": "return", "value": return_value},
                        }
                    ],
                },
            }
        )

    if not verified_functions:
        raise UnsupportedTypedCoreHir("$.functions: no contracted functions")
    missing = sorted(type_id for type_id in used_types if type_id not in source_types)
    if missing:
        raise UnsupportedTypedCoreHir(f"$.types: missing reachable type ids {missing}")

    verified_types = [
        base._verified_type(source_types[type_id], f"$.types[id={type_id}]")
        for type_id in sorted(used_types)
    ]
    result = {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": "arukellt-typed-corehir-contract-converter-v2",
        "module": source["module"],
        "target_profile": copy.deepcopy(source["target_profile"]),
        "types": verified_types,
        "functions": verified_functions,
    }
    return validate_verified_core(result)


__all__ = ["UnsupportedTypedCoreHir", "convert_document"]
