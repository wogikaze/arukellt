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

from proof.smtlib_typed_v1 import UnsupportedTypedVerifiedCore, generate_typed_smtlib  # noqa: E402
from proof.typed_corehir_typed_convert import convert_typed_document  # noqa: E402


class TypedSmtLibTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        source = json.loads((ROOT / "tests" / "proof" / "typed-corehir.json").read_text())
        cls.document = convert_typed_document(source)

    def test_generates_after_typed_admission(self) -> None:
        rendered = generate_typed_smtlib(copy.deepcopy(self.document))
        self.assertIn("(set-logic QF_NIA)", rendered)
        self.assertIn("(check-sat)", rendered)

    def test_rejects_operator_type_mismatch_before_rendering(self) -> None:
        document = copy.deepcopy(self.document)
        expression = document["functions"][0]["contracts"][0]["expression"]
        expression["kind"] = "add"
        with self.assertRaisesRegex(UnsupportedTypedVerifiedCore, "arithmetic type mismatch"):
            generate_typed_smtlib(document)

    def test_rejects_parameter_local_mismatch_before_rendering(self) -> None:
        document = copy.deepcopy(self.document)
        document["functions"][0]["locals"][0]["storage"] = "local"
        with self.assertRaisesRegex(UnsupportedTypedVerifiedCore, "parameter .* mismatch"):
            generate_typed_smtlib(document)


if __name__ == "__main__":
    unittest.main()
