from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.smtlib_typed_v7 import generate_typed_smtlib
from proof.typed_corehir_v1_scalar_v3 import GENERATOR, upgrade_scalar_document
from proof.typed_corehir_v3 import validate_document
from proof.typed_corehir_v3_convert import CONVERTER, convert_document


def source_document() -> dict:
    value = json.loads((ROOT / "tests" / "proof" / "typed-corehir.json").read_text(encoding="utf-8"))
    value["generator"] = "arukellt-selfhost"
    function = value["functions"][0]
    expressions = function["body"]["expressions"]
    expressions.append(copy.deepcopy(expressions[1]))
    expressions[-1]["id"] = 3
    function["contracts"][0]["expression_id"] = 2
    expressions[2]["children"] = [1, 3]
    return value


class TypedCoreHIRV1ScalarV3Tests(unittest.TestCase):
    def test_scalar_selfhost_source_upgrades_and_reaches_phase7_smt(self):
        upgraded = upgrade_scalar_document(source_document())
        self.assertEqual(upgraded["schema_version"], 3)
        self.assertEqual(upgraded["generator"], GENERATOR)
        self.assertEqual(upgraded["target_profile"]["integer_model"], "machine")
        self.assertEqual(upgraded["target_profile"]["overflow"], "checked")
        self.assertEqual(upgraded["target_profile"]["floating_point"], "unsupported")
        self.assertEqual(upgraded["proof_memory"], {"model": "arukellt-readonly-heap-v1", "references": []})
        self.assertNotIn(2, upgraded["functions"][0]["body"]["expressions"][0]["children"])
        validate_document(upgraded)

        converted = convert_document(upgraded)
        self.assertEqual(converted["generator"], CONVERTER)
        rendered = generate_typed_smtlib(converted)
        self.assertIn("(check-sat)", rendered)
        self.assertIn("machine", rendered)

    def test_overflow_capable_source_arithmetic_is_rejected(self):
        source = source_document()
        expression = source["functions"][0]["body"]["expressions"][2]
        expression["text"] = "+"
        expression["type_id"] = 1
        with self.assertRaisesRegex(ValueError, "overflow-capable"):
            upgrade_scalar_document(source)

    def test_used_float_type_is_rejected(self):
        source = source_document()
        source["types"].append({
            "id": 5,
            "kind": "float",
            "name": "f32",
            "value_type": "f32",
            "representation": {"kind": "scalar", "wasm": ["f32"], "nullable": False, "size_bytes": 4, "align_bytes": 4},
        })
        function = source["functions"][0]
        function["signature"]["return_type_id"] = 5
        function["abi"]["results"] = [{"type_id": 5, "passing": "value", "wasm": ["f32"]}]
        function["body"]["expressions"][0]["type_id"] = 5
        function["body"]["expressions"][1]["type_id"] = 5
        with self.assertRaisesRegex(ValueError, "scalar bridge rejects"):
            upgrade_scalar_document(source)

    def test_selfhost_char_typeid_is_not_reinterpreted_as_signed_integer(self):
        source = source_document()
        source["types"].append({
            "id": 3,
            "kind": "integer",
            "name": "char",
            "bits": 32,
            "signed": True,
            "value_type": "i32",
            "representation": {"kind": "scalar", "wasm": ["i32"], "nullable": False, "size_bytes": 4, "align_bytes": 4},
        })
        function = source["functions"][0]
        function["signature"]["return_type_id"] = 3
        function["abi"]["results"] = [{"type_id": 3, "passing": "value", "wasm": ["i32"]}]
        function["body"]["expressions"][0]["type_id"] = 3
        function["body"]["expressions"][1]["type_id"] = 3
        with self.assertRaisesRegex(ValueError, "selfhost TypeId 3"):
            upgrade_scalar_document(source)


if __name__ == "__main__":
    unittest.main()
