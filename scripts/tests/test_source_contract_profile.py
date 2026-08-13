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

from proof.normalize_source_contract_profile import (  # noqa: E402
    UnsupportedSourceContractProfile,
    normalize_document,
)
from proof.typed_corehir_contract_convert import convert_document  # noqa: E402


class SourceContractProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        typed = json.loads((ROOT / "tests" / "proof" / "typed-corehir.json").read_text())
        cls.verified = convert_document(typed)

    def test_comparison_only_contract_normalizes(self) -> None:
        document = copy.deepcopy(self.verified)
        document["target_profile"] = {
            "integer_model": "machine",
            "overflow": "compiler-current",
            "floating_point": "ieee754",
            "pointer_width": 32,
        }
        normalized = normalize_document(document)
        self.assertEqual(normalized["target_profile"], {
            "integer_model": "mathematical",
            "overflow": "checked",
            "floating_point": "unsupported",
            "pointer_width": 32,
        })
        self.assertIn("comparison-profile-normalizer-v1", normalized["generator"])

    def test_machine_arithmetic_contract_fails_closed(self) -> None:
        document = copy.deepcopy(self.verified)
        expression = document["functions"][0]["contracts"][0]["expression"]
        expression["kind"] = "add"
        with self.assertRaisesRegex(
            UnsupportedSourceContractProfile,
            "machine arithmetic cannot be normalized",
        ):
            normalize_document(document)


if __name__ == "__main__":
    unittest.main()
