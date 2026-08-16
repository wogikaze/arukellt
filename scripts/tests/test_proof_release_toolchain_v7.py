from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "gen" / "prepare-proof-release-toolchain-v7.py"
SPEC = importlib.util.spec_from_file_location("prepare_proof_release_toolchain_v7", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

from scripts.tests.test_typed_corehir_v3_machine_memory import source_document  # noqa: E402


def base_toolchain() -> dict:
    return {
        "translator": {"name": "arukellt-typed-verified-core-smtlib", "version": "5"},
        "semantic_profile": {
            "integer_model": "mathematical",
            "overflow": "checked",
            "floating_point": "unsupported",
            "memory": "pure-values",
            "aggregate_model": "pure-algebraic-datatypes",
        },
        "trusted_components": [],
    }


class ProofReleaseToolchainV7Tests(unittest.TestCase):
    def test_v1_source_does_not_claim_phase67_semantics(self):
        document = base_toolchain()
        active = MODULE._apply_source_profile(document, 1)
        self.assertFalse(active)
        self.assertEqual(document["translator"]["version"], "5")
        self.assertEqual(document["semantic_profile"]["memory"], "pure-values")
        self.assertEqual(document["semantic_profile"]["source_schema_version"], 1)
        self.assertTrue(document["semantic_profile"]["phase67_available"])
        self.assertFalse(document["semantic_profile"]["phase67_active"])
        self.assertNotIn("memory_model", document["semantic_profile"])
        self.assertNotIn("machine_integer_model", document["semantic_profile"])

    def test_v3_source_activates_machine_integer_and_readonly_heap_profile(self):
        document = base_toolchain()
        active = MODULE._apply_source_profile(document, 3)
        self.assertTrue(active)
        self.assertEqual(document["translator"]["version"], "7")
        profile = document["semantic_profile"]
        self.assertEqual(profile["source_schema_version"], 3)
        self.assertTrue(profile["phase67_active"])
        self.assertEqual(profile["integer_model"], "machine")
        self.assertEqual(profile["overflow"], "checked")
        self.assertEqual(profile["memory_model"], "arukellt-readonly-heap-v1")
        self.assertEqual(profile["memory_encoding"], "arukellt-readonly-heap-smt-v1")
        self.assertEqual(profile["machine_integer_model"], "arukellt-machine-int-v1")
        self.assertEqual(profile["capability_profile"], "proof-phases-0-7@3")

    def test_v3_source_is_semantically_validated_before_activation(self):
        source = source_document()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "typed-corehir.json"
            path.write_text(json.dumps(source), encoding="utf-8")
            version, validated = MODULE._source_version(path)
            self.assertEqual(version, 3)
            self.assertEqual(validated["proof_memory"]["model"], "arukellt-readonly-heap-v1")

            invalid = copy.deepcopy(source)
            invalid["proof_memory"]["references"] = []
            path.write_text(json.dumps(invalid), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "exactly all reference TypeIds"):
                MODULE._source_version(path)

    def test_unknown_source_version_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "typed-corehir.json"
            path.write_text(
                json.dumps({"schema": "arukellt-typed-corehir", "schema_version": 4}),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "unsupported TypedCoreHIR source version"):
                MODULE._source_version(path)

    def test_release_command_passes_emitted_source_to_toolchain_builder(self):
        text = (ROOT / "scripts" / "run" / "proof-required-release.sh").read_text(encoding="utf-8")
        call = text.split("python3 scripts/gen/prepare-proof-release-toolchain-v7.py", 1)[1]
        self.assertIn('--typed-corehir "$PROOF_ROOT/typed-corehir.json"', call)


if __name__ == "__main__":
    unittest.main()
