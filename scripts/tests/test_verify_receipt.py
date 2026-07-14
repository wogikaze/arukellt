from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "gen" / "write-verify-receipt.py"
SPEC = importlib.util.spec_from_file_location("write_receipt", SCRIPT)
assert SPEC and SPEC.loader
write_receipt = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = write_receipt
SPEC.loader.exec_module(write_receipt)


SAMPLE_OUTPUT = """\
[bg] Running background checks in parallel
T3 WASM validation: 2 pass, 1 validate-fail, 0 compile-fail, 1 skip (total 4)
  VALIDATE FAIL: generics_v1/trait_dispatch_stdlib.ark — error: func 41 failed to validate
Total checks: 165
Passed: 163
Skipped: 1
Failed: 1
[fixtures] Running selfhost fixture parity
  FAIL: enums/nested_enum.ark (current wasm invalid, pinned OK)
  skip: enums/tuple_variant.ark (both wasms invalid)
  FAIL: examples/rpn_repl.ark (current wasm invalid)
fixture-parity: PASS=804 FAIL=2 SKIP=1
[size] Checking hello.wasm binary size gate
  ✓ hello.wasm binary size: 610 bytes
[wat] Running WAT roundtrip
  FAIL: wat_roundtrip_fail.ark (wat2wasm failed)
WAT roundtrip summary: PASS=10 FAIL=1 SKIP=0
[component] Component interop smoke test
  ✗ component interop: bool-logic (wasmtime)
  ✗ component interop: calculator (wasmtime)
[cli-parity] Checking selfhost CLI surface
  pass: --version (matches golden)
  pass: --help (matches golden)
  FAIL: --help (drifts from golden — update)
  FAIL: compose --validate (exit=1, output='error')
cli-parity: PASS=2 FAIL=2
[diag-parity] Checking 3 diag: fixtures against committed .diag goldens
  pass: target_gating/t0_basic.ark
  skip: target_gating/t1_import_http.ark (missing diag)
  FAIL: target_gating/t2_unreachable.ark (selfhost: pattern 'unreachable' not found)
diag-parity: PASS=1 SKIP=1 FAIL=1
[selfhost-fixpoint] Fixpoint gate (full verify)
  sha256(s2) = 2133ac1fc3b4406b128002ac770954f8f81e9b923f0e32413b66c2365c7d954d
  sha256(s3) = 9080f639727cba3d1371c4a14a4ea0cd9fe49c3a81429ad43710609b993e5ff7
✗ selfhost fixpoint not reached (exit 1)
"""

T3_REPORT = {
    "schema_version": 1,
    "pass_count": 2,
    "fail_validate": 1,
    "fail_compile": 0,
    "skip_count": 1,
    "items": [
        {"fixture": "passing/pass_a.ark", "status": "pass", "detail": ""},
        {"fixture": "passing/pass_b.ark", "status": "pass", "detail": ""},
        {
            "fixture": "generics_v1/trait_dispatch_stdlib.ark",
            "status": "validate-fail",
            "detail": "error: func 41 failed to validate",
        },
        {
            "fixture": "operators/mixed_types.ark",
            "status": "skip",
            "detail": "compile-time skip",
        },
    ],
}

BAD_LOG = """\
[diag-parity] Checking 1 diag: fixtures against committed .diag goldens
  pass: target_gating/t0_basic.ark
diag-parity: PASS=2 SKIP=0 FAIL=0
"""


def _build_sample_receipt() -> dict:
    return write_receipt.build_receipt(
        SAMPLE_OUTPUT,
        started_at="2026-07-14T14:02:00+00:00",
        exit_status=1,
        verified_commit="testabc1",
        t3_report=T3_REPORT,
    )


class VerifyReceiptTest(unittest.TestCase):
    def test_schema_version_and_metadata(self) -> None:
        receipt = _build_sample_receipt()
        self.assertEqual(receipt["schema_version"], 2)
        self.assertEqual(receipt["verified_commit"], "testabc1")
        self.assertEqual(receipt["exit_status"], 1)
        self.assertEqual(receipt["status"], "fail")
        self.assertIn("generated_at", receipt)
        self.assertIn("finished_at", receipt)
        self.assertIn("started_at", receipt)

    def test_summary_counts(self) -> None:
        receipt = _build_sample_receipt()
        summary = receipt["summary"]
        self.assertEqual(summary["checks_total"], 9)
        self.assertEqual(summary["checks_passed"], 1)
        self.assertEqual(summary["checks_failed"], 8)
        self.assertEqual(summary["checks_skipped"], 0)
        self.assertEqual(summary["total_items"], 17)
        self.assertEqual(summary["item_failures"], 9)
        self.assertEqual(summary["item_skips"], 3)
        self.assertEqual(summary["t3_pass"], 2)
        self.assertEqual(summary["t3_fail"], 1)
        self.assertEqual(summary["t3_skip"], 1)
        self.assertEqual(summary["fixture_pass"], 804)
        self.assertEqual(summary["fixture_fail"], 2)
        self.assertEqual(summary["fixture_skip"], 1)
        self.assertEqual(summary["incidents"], [])

    def _check(self, receipt: dict, check_id: str) -> dict:
        for check in receipt["checks"]:
            if check["check_id"] == check_id:
                return check
        raise AssertionError(f"check {check_id!r} not found")

    def test_size_aggregate_only(self) -> None:
        receipt = _build_sample_receipt()
        check = self._check(receipt, "size")
        self.assertEqual(check["identity_coverage"], "aggregate_only")
        self.assertEqual(check["owner_issue"], "422")
        self.assertEqual(check["pass_count"], 1)
        self.assertEqual(check["fail_count"], 0)
        self.assertEqual(check["skip_count"], 0)
        self.assertEqual(check["result"], "pass")
        self.assertEqual(check["items"], [])

    def test_quick_checks_aggregate_only(self) -> None:
        receipt = _build_sample_receipt()
        check = self._check(receipt, "quick_checks")
        self.assertEqual(check["identity_coverage"], "aggregate_only")
        self.assertEqual(check["owner_issue"], "808")
        self.assertEqual(check["pass_count"], 163)
        self.assertEqual(check["fail_count"], 1)
        self.assertEqual(check["skip_count"], 1)
        self.assertEqual(check["result"], "fail")
        self.assertEqual(check["items"], [])

    def test_t3_wasm_validate_full(self) -> None:
        receipt = _build_sample_receipt()
        check = self._check(receipt, "t3_wasm_validate")
        self.assertEqual(check["identity_coverage"], "full")
        self.assertEqual(check["owner_issue"], "808")
        self.assertEqual(check["pass_count"], 2)
        self.assertEqual(check["fail_count"], 1)
        self.assertEqual(check["skip_count"], 1)
        self.assertEqual(check["result"], "fail")

        pass_items = [i for i in check["items"] if i["result"] == "pass"]
        fail_items = [i for i in check["items"] if i["result"] == "fail"]
        skip_items = [i for i in check["items"] if i["result"] == "skip"]

        self.assertEqual(len(pass_items), 2)
        self.assertEqual(len(fail_items), 1)
        self.assertEqual(len(skip_items), 1)

        self.assertTrue(all(i["owner_issue"] == "808" for i in pass_items))
        self.assertTrue(all(i["owner_issue"] == "808" for i in fail_items))
        self.assertEqual(skip_items[0]["owner_issue"], "815")

    def test_fixture_parity_nonpass_full(self) -> None:
        receipt = _build_sample_receipt()
        check = self._check(receipt, "fixture_parity")
        self.assertEqual(check["identity_coverage"], "nonpass_full")
        self.assertEqual(check["owner_issue"], "807")
        self.assertEqual(check["pass_count"], 804)
        self.assertEqual(check["fail_count"], 2)
        self.assertEqual(check["skip_count"], 1)

        pass_items = [i for i in check["items"] if i["result"] == "pass"]
        fail_items = [i for i in check["items"] if i["result"] == "fail"]
        skip_items = [i for i in check["items"] if i["result"] == "skip"]

        self.assertEqual(len(pass_items), 0)
        self.assertEqual(len(fail_items), 2)
        self.assertEqual(len(skip_items), 1)
        self.assertTrue(all(i["owner_issue"] == "807" for i in fail_items))
        self.assertTrue(all(i["owner_issue"] == "807" for i in skip_items))

    def test_wat_roundtrip_partial(self) -> None:
        receipt = _build_sample_receipt()
        check = self._check(receipt, "wat_roundtrip")
        self.assertEqual(check["identity_coverage"], "partial")
        self.assertEqual(check["owner_issue"], "809")
        self.assertEqual(check["pass_count"], 10)
        self.assertEqual(check["fail_count"], 1)
        self.assertEqual(check["skip_count"], 0)

        fail_items = [i for i in check["items"] if i["result"] == "fail"]
        self.assertEqual(len(fail_items), 1)
        self.assertEqual(fail_items[0]["item_id"], "wat_roundtrip_fail.ark")
        self.assertEqual(fail_items[0]["owner_issue"], "809")

    def test_component_interop_full(self) -> None:
        receipt = _build_sample_receipt()
        check = self._check(receipt, "component_interop")
        self.assertEqual(check["identity_coverage"], "full")
        self.assertEqual(check["owner_issue"], "810")
        self.assertEqual(check["pass_count"], 0)
        self.assertEqual(check["fail_count"], 2)

        fail_items = [i for i in check["items"] if i["result"] == "fail"]
        self.assertEqual(len(fail_items), 2)
        self.assertEqual({i["item_id"] for i in fail_items}, {"bool-logic", "calculator"})
        self.assertTrue(all(i["owner_issue"] == "810" for i in fail_items))

    def test_cli_parity_full(self) -> None:
        receipt = _build_sample_receipt()
        check = self._check(receipt, "cli_parity")
        self.assertEqual(check["identity_coverage"], "full")
        self.assertEqual(check["owner_issue"], "811")
        self.assertEqual(check["pass_count"], 2)
        self.assertEqual(check["fail_count"], 2)

        pass_items = [i for i in check["items"] if i["result"] == "pass"]
        fail_items = [i for i in check["items"] if i["result"] == "fail"]

        self.assertEqual(len(pass_items), 2)
        self.assertEqual(len(fail_items), 2)
        self.assertTrue(all(i["owner_issue"] == "811" for i in pass_items))
        self.assertTrue(all(i["owner_issue"] == "811" for i in fail_items))

    def test_diag_parity_full(self) -> None:
        receipt = _build_sample_receipt()
        check = self._check(receipt, "diag_parity")
        self.assertEqual(check["identity_coverage"], "full")
        self.assertEqual(check["owner_issue"], "812")
        self.assertEqual(check["pass_count"], 1)
        self.assertEqual(check["fail_count"], 1)
        self.assertEqual(check["skip_count"], 1)

        pass_items = [i for i in check["items"] if i["result"] == "pass"]
        fail_items = [i for i in check["items"] if i["result"] == "fail"]
        skip_items = [i for i in check["items"] if i["result"] == "skip"]

        self.assertEqual(len(pass_items), 1)
        self.assertEqual(len(fail_items), 1)
        self.assertEqual(len(skip_items), 1)
        self.assertEqual(pass_items[0]["owner_issue"], "812")
        self.assertEqual(fail_items[0]["owner_issue"], "812")
        self.assertEqual(skip_items[0]["owner_issue"], "815")

    def test_fixpoint_aggregate_only_evidence(self) -> None:
        receipt = _build_sample_receipt()
        check = self._check(receipt, "fixpoint")
        self.assertEqual(check["identity_coverage"], "aggregate_only")
        self.assertEqual(check["owner_issue"], "813")
        self.assertEqual(check["pass_count"], 0)
        self.assertEqual(check["fail_count"], 1)
        self.assertEqual(check["items"], [])
        self.assertEqual(
            check["evidence"],
            {
                "s2_hash": "2133ac1fc3b4406b128002ac770954f8f81e9b923f0e32413b66c2365c7d954d",
                "s3_hash": "9080f639727cba3d1371c4a14a4ea0cd9fe49c3a81429ad43710609b993e5ff7",
            },
        )

    def test_invariants_detect_coverage_mismatch(self) -> None:
        receipt = write_receipt.build_receipt(
            BAD_LOG,
            t3_report=T3_REPORT,
            verified_commit="testabc1",
        )
        incidents = receipt["summary"]["incidents"]
        self.assertTrue(incidents)
        self.assertTrue(any("diag_parity" in i for i in incidents))

    def test_main_returns_nonzero_on_incidents(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            input_path = Path(tmpdir) / "bad.log"
            output_path = Path(tmpdir) / "out.json"
            input_path.write_text(BAD_LOG, encoding="utf-8")

            rc = write_receipt.main(
                ["--input", str(input_path), "--output", str(output_path)]
            )
            self.assertEqual(rc, 1)
            self.assertTrue(output_path.is_file())


if __name__ == "__main__":
    unittest.main()
