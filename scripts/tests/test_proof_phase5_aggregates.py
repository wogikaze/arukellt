from __future__ import annotations

import copy

from scripts.tests.test_typed_corehir_v2_aggregates import document as source_document
from proof.typed_corehir_v2_convert import convert_document


def _aggregate_source() -> dict:
    value = source_document()
    rep = {"wasm": [], "nullable": False, "size_bytes": 0, "align_bytes": 1}
    value["types"][3] = {
        "id": 10,
        "kind": "tuple",
        "name": "Pair",
        "elements": [1, 2],
        "representation": copy.deepcopy(rep),
    }
    value["types"].extend([
        {
            "id": 11,
            "kind": "struct",
            "name": "Record",
            "fields": [{"name": "value", "type_id": 1}, {"name": "ok", "type_id": 2}],
            "representation": copy.deepcopy(rep),
        },
        {
            "id": 12,
            "kind": "enum",
            "name": "OptionI32",
            "variants": [
                {"name": "None", "discriminant": 0, "payload_type_ids": []},
                {"name": "Some", "discriminant": 1, "payload_type_ids": [1]},
            ],
            "representation": copy.deepcopy(rep),
        },
        {
            "id": 13,
            "kind": "enum",
            "name": "ResultI32",
            "variants": [
                {"name": "Ok", "discriminant": 0, "payload_type_ids": [1]},
                {"name": "Err", "discriminant": 1, "payload_type_ids": [1]},
            ],
            "representation": copy.deepcopy(rep),
        },
    ])
    function = value["functions"][0]
    function["signature"]["return_type_id"] = 12
    function["abi"]["results"][0]["type_id"] = 12
    expressions = function["body"]["expressions"]
    expressions[1]["type_id"] = 12
    expressions[2]["type_id"] = 12
    expressions[4]["type_id"] = 12
    expressions[5]["type_id"] = 12
    return value


def document() -> dict:
    return convert_document(_aggregate_source())


__all__ = ["document"]
