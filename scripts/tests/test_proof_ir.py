"""Unit tests for the versioned Proof IR boundary."""

from __future__ import annotations

import json
import sys
import unittest
from copy import deepcopy
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[2]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from scripts.proof.ir import ValidationError, validate_document  # noqa: E402


class TestProofIr(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.valid = json.loads(
            (_ROOT / "tests" / "proof-ir" / "basic.json").read_text(encoding="utf-8")
        )

    def test_valid_fixture(self) -> None:
        self.assertIs(validate_document(deepcopy(self.valid))["functions"][0]["id"], 0)

    def test_rejects_unknown_schema_version(self) -> None:
        document = deepcopy(self.valid)
        document["schema_version"] = 2
        with self.assertRaisesRegex(ValidationError, "expected 1"):
            validate_document(document)

    def test_rejects_duplicate_function_ids(self) -> None:
        document = deepcopy(self.valid)
        duplicate = deepcopy(document["functions"][0])
        duplicate["name"] = "abs_again"
        document["functions"].append(duplicate)
        with self.assertRaisesRegex(ValidationError, "duplicate function id"):
            validate_document(document)

    def test_ensures_requires_result_name(self) -> None:
        document = deepcopy(self.valid)
        del document["functions"][0]["contracts"][0]["result_name"]
        with self.assertRaisesRegex(ValidationError, "requires result_name"):
            validate_document(document)

    def test_result_name_is_ensures_only(self) -> None:
        document = deepcopy(self.valid)
        contract = document["functions"][0]["contracts"][0]
        contract["kind"] = "requires"
        with self.assertRaisesRegex(ValidationError, "only valid for ensures"):
            validate_document(document)

    def test_rejects_reversed_span(self) -> None:
        document = deepcopy(self.valid)
        document["functions"][0]["span"] = {
            "file": "basic.ark",
            "start": 10,
            "end": 4,
        }
        with self.assertRaisesRegex(ValidationError, "end must be >= start"):
            validate_document(document)


if __name__ == "__main__":
    unittest.main()
