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

from proof.smtlib_v1 import UnsupportedVerifiedCore, generate_smtlib  # noqa: E402


class SmtlibV1Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.subject = json.loads((ROOT / "tests" / "proof" / "verified-core.json").read_text())

    def test_identity_contract_generates_unsat_obligation(self) -> None:
        output = generate_smtlib(copy.deepcopy(self.subject))
        self.assertIn("(set-logic QF_LIA)", output)
        self.assertIn("(assert (= f0_identity_result f0_identity_local_0_x))", output)
        self.assertIn("(assert (not (>= f0_identity_result f0_identity_local_0_x)))", output)
        self.assertEqual(output.count("(check-sat)"), 1)

    def test_requires_is_assumed_before_negated_ensures(self) -> None:
        document = copy.deepcopy(self.subject)
        function = document["functions"][0]
        function["contracts"].insert(
            0,
            {
                "kind": "requires",
                "expression": {
                    "id": 10,
                    "kind": "ge",
                    "type_id": 2,
                    "operands": [
                        {"id": 11, "kind": "local", "type_id": 1, "local_id": 0},
                        {"id": 12, "kind": "constant", "type_id": 1, "value": 0},
                    ],
                },
            },
        )
        output = generate_smtlib(document)
        requirement = "(assert (>= f0_identity_local_0_x 0))"
        negated_ensure = "(assert (not (>= f0_identity_result f0_identity_local_0_x)))"
        self.assertLess(output.index(requirement), output.index(negated_ensure))

    def test_body_instructions_fail_closed(self) -> None:
        document = copy.deepcopy(self.subject)
        document["functions"][0]["body"]["blocks"][0]["instructions"] = [
            {"kind": "opaque"}
        ]
        with self.assertRaisesRegex(UnsupportedVerifiedCore, "instructions unsupported"):
            generate_smtlib(document)

    def test_unknown_contract_operator_fails_closed(self) -> None:
        document = copy.deepcopy(self.subject)
        document["functions"][0]["contracts"][0]["expression"]["kind"] = "magic"
        with self.assertRaisesRegex(UnsupportedVerifiedCore, "unsupported proof expression"):
            generate_smtlib(document)

    def test_machine_integer_profile_fails_closed(self) -> None:
        document = copy.deepcopy(self.subject)
        document["target_profile"]["integer_model"] = "machine"
        with self.assertRaisesRegex(UnsupportedVerifiedCore, "only mathematical"):
            generate_smtlib(document)


if __name__ == "__main__":
    unittest.main()
