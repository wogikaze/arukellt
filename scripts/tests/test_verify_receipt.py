from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "gen" / "write-verify-receipt.py"
SPEC = importlib.util.spec_from_file_location("write_receipt", SCRIPT)
assert SPEC and SPEC.loader
write_receipt = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = write_receipt
SPEC.loader.exec_module(write_receipt)


SAMPLE_OUTPUT = """\x1b[1;33m[manifest] Checking...\x1b[0m
\x1b[0;32m✓ Fixture manifest completeness (2692 entries)\x1b[0m
\x1b[0;32m✓ false-done close-gate enforcement\x1b[0m
  FAIL: enums/nested_enum.ark (current wasm invalid, pinned OK)
  skip: enums/tuple_variant.ark (both wasms invalid)
  FAIL: examples/rpn_repl.ark (current wasm invalid)
\x1b[1;33mfixture-parity: PASS=804 FAIL=367 SKIP=417\x1b[0m
\x1b[0;31m✗ T3 fixture WASM validation gate (#686)\x1b[0m
\x1b[0;31m✗ WAT roundtrip (wasm2wat ⇄ wat2wasm)\x1b[0m
\x1b[0;31m✗ component interop: bool-logic (wasmtime)\x1b[0m
\x1b[0;31m✗ component interop: calculator (wasmtime)\x1b[0m
\x1b[1;33mcli-parity: PASS=17 FAIL=2\x1b[0m
  FAIL: --help (drifts from golden — update)
  FAIL: compose --validate (exit=1, output='error')
\x1b[1;33mdiag-parity: PASS=29 SKIP=26 FAIL=3\x1b[0m
  FAIL: target_gating/t1_import_http.ark (selfhost: pattern not found)
\x1b[0;31m✗ selfhost fixpoint not reached (exit 1)\x1b[0m
  sha256(s2) = 2133ac1fc3b4406b128002ac770954f8f81e9b923f0e32413b66c2365c7d954d
  sha256(s3) = 9080f639727cba3d1371c4a14a4ea0cd9fe49c3a81429ad43710609b993e5ff7
========================================
Summary
========================================
Total checks: 165
Passed: 163
Skipped: 0
Failed: 2
"""


class VerifyReceiptTest(unittest.TestCase):
    def test_parse_fixture_parity(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        agg = {c["check_id"]: c for c in receipt["aggregate_checks"]}
        self.assertIn("fixture_parity", agg)
        self.assertEqual(agg["fixture_parity"]["pass_count"], 804)
        self.assertEqual(agg["fixture_parity"]["fail_count"], 367)
        self.assertEqual(agg["fixture_parity"]["skip_count"], 417)
        self.assertEqual(agg["fixture_parity"]["owner_issue"], "807")

    def test_parse_t3_wasm_validate(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        agg = {c["check_id"]: c for c in receipt["aggregate_checks"]}
        self.assertIn("t3_wasm_validate", agg)
        self.assertEqual(agg["t3_wasm_validate"]["result"], "fail")
        self.assertEqual(agg["t3_wasm_validate"]["owner_issue"], "808")

    def test_parse_wat_roundtrip(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        agg = {c["check_id"]: c for c in receipt["aggregate_checks"]}
        self.assertIn("wat_roundtrip", agg)
        self.assertEqual(agg["wat_roundtrip"]["result"], "fail")
        self.assertEqual(agg["wat_roundtrip"]["owner_issue"], "809")

    def test_parse_cli_parity(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        agg = {c["check_id"]: c for c in receipt["aggregate_checks"]}
        self.assertIn("cli_parity", agg)
        self.assertEqual(agg["cli_parity"]["fail_count"], 2)
        self.assertEqual(agg["cli_parity"]["owner_issue"], "811")

    def test_parse_diag_parity(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        agg = {c["check_id"]: c for c in receipt["aggregate_checks"]}
        self.assertIn("diag_parity", agg)
        self.assertEqual(agg["diag_parity"]["fail_count"], 3)
        self.assertEqual(agg["diag_parity"]["owner_issue"], "812")

    def test_parse_fixpoint(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        agg = {c["check_id"]: c for c in receipt["aggregate_checks"]}
        self.assertIn("fixpoint", agg)
        self.assertEqual(agg["fixpoint"]["result"], "fail")
        self.assertEqual(agg["fixpoint"]["owner_issue"], "813")

    def test_parse_individual_fixture_items(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        items = [i for i in receipt["items"] if i["check_id"] == "fixture_parity"]
        fail_items = [i for i in items if i["result"] == "fail"]
        skip_items = [i for i in items if i["result"] == "skip"]
        self.assertGreater(len(fail_items), 0)
        self.assertGreater(len(skip_items), 0)
        self.assertEqual(fail_items[0]["item_id"], "enums/nested_enum.ark")
        self.assertEqual(fail_items[0]["owner_issue"], "807")
        self.assertEqual(skip_items[0]["item_id"], "enums/tuple_variant.ark")
        self.assertEqual(skip_items[0]["owner_issue"], "815")

    def test_parse_component_interop_items(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        items = [i for i in receipt["items"] if i["check_id"] == "component_interop"]
        self.assertEqual(len(items), 2)
        self.assertEqual(items[0]["item_id"], "bool-logic")
        self.assertEqual(items[0]["owner_issue"], "810")

    def test_parse_cli_parity_items(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        items = [i for i in receipt["items"] if i["check_id"] == "cli_parity"]
        self.assertEqual(len(items), 2)
        self.assertIn("--help", [i["item_id"] for i in items])
        self.assertIn("compose --validate", [i["item_id"] for i in items])

    def test_parse_diag_parity_items(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        items = [i for i in receipt["items"] if i["check_id"] == "diag_parity"]
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0]["item_id"], "target_gating/t1_import_http.ark")

    def test_parse_fixpoint_hashes(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        items = [i for i in receipt["items"] if i["check_id"] == "fixpoint"]
        s2 = [i for i in items if i["item_id"] == "s2_hash"]
        s3 = [i for i in items if i["item_id"] == "s3_hash"]
        self.assertEqual(len(s2), 1)
        self.assertEqual(len(s3), 1)
        self.assertEqual(s2[0]["detail"], "2133ac1fc3b4406b128002ac770954f8f81e9b923f0e32413b66c2365c7d954d")

    def test_schema_version_and_metadata(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        self.assertEqual(receipt["schema_version"], 2)
        self.assertIn("verified_commit", receipt)
        self.assertIn("finished_at", receipt)
        self.assertIn("command", receipt)

    def test_aggregate_and_items_separate(self) -> None:
        receipt = write_receipt.parse_receipt(SAMPLE_OUTPUT)
        # aggregate_checks should not contain individual fixture IDs
        for c in receipt["aggregate_checks"]:
            self.assertNotIn("item_id", c)
        # items should not contain aggregate counts
        for item in receipt["items"]:
            self.assertNotIn("pass_count", item)
            self.assertNotIn("fail_count", item)


if __name__ == "__main__":
    unittest.main()
