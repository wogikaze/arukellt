from __future__ import annotations
import copy, sys, unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path: sys.path.insert(0, str(SCRIPTS))
from proof.smtlib_typed_v5 import generate_typed_smtlib
from proof.typed_admission_v5_strict import validate_typed_document
from scripts.tests.test_proof_phase5_aggregates import document

class ProofPhase5StrictTests(unittest.TestCase):
    def test_strict_entrypoint_renders_all_aggregate_families(self):
        rendered = generate_typed_smtlib(document())
        self.assertIn("aggregate-encoding=arukellt-smt-datatype-v1", rendered)
        for token in ("A10", "A11", "A12", "A13", "a10_s0", "a12_c1", "a12_v1_p0"):
            self.assertIn(token, rendered)
    def test_type_names_do_not_change_solver_input(self):
        value = document(); first = generate_typed_smtlib(value)
        for entry in value["types"]:
            if entry["id"] >= 10: entry["name"] = "Renamed" + str(entry["id"])
        self.assertEqual(first, generate_typed_smtlib(value))
    def test_recursive_aggregate_rejects_before_smt(self):
        value = document(); value["types"][3]["elements"] = [10]
        with self.assertRaisesRegex(ValueError, "recursive aggregate"):
            validate_typed_document(value)

if __name__ == "__main__": unittest.main()
