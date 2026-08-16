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

from scripts.tests.test_typed_corehir_v1_scalar_v3 import source_document as scalar_v1_source  # noqa: E402
from scripts.tests.test_typed_corehir_v3_machine_memory import source_document as v3_memory_source  # noqa: E402
from proof.typed_corehir_v1_scalar_v3 import upgrade_scalar_document  # noqa: E402


def base_toolchain() -> dict:
    return {
        "translator": {"name": "arukellt-typed-verified-core-smtlib", "version": "5", "executable": "smtlib_typed_v1.py"},
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
        self.assertEqual(document["semantic_profile"]["raw_source_schema_version"], 1)
        self.assertFalse(document["semantic_profile"]["source_upgrade_active"])
        self.assertTrue(document["semantic_profile"]["phase67_available"])
        self.assertFalse(document["semantic_profile"]["phase67_active"])
        self.assertNotIn("memory_model", document["semantic_profile"])
        self.assertNotIn("machine_integer_model", document["semantic_profile"])

    def test_v3_source_activates_v7_translator_and_machine_memory_profile(self):
        document = base_toolchain()
        active = MODULE._apply_source_profile(document, 3, 1)
        self.assertTrue(active)
        self.assertEqual(document["translator"]["name"], "arukellt-phase7-smt-adapter")
        self.assertEqual(document["translator"]["version"], "7")
        self.assertEqual(document["translator"]["executable"], "smtlib_typed_v7.py")
        profile = document["semantic_profile"]
        self.assertEqual(profile["source_schema_version"], 3)
        self.assertEqual(profile["raw_source_schema_version"], 1)
        self.assertTrue(profile["source_upgrade_active"])
        self.assertTrue(profile["phase67_active"])
        self.assertEqual(profile["integer_model"], "machine")
        self.assertEqual(profile["overflow"], "checked")
        self.assertEqual(profile["memory_model"], "arukellt-readonly-heap-v1")
        self.assertEqual(profile["memory_encoding"], "arukellt-readonly-heap-smt-v1")
        self.assertEqual(profile["machine_integer_model"], "arukellt-machine-int-v1")
        self.assertEqual(profile["capability_profile"], "proof-phases-0-7@3")

    def test_v3_source_is_semantically_validated_before_activation(self):
        source = v3_memory_source()
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

    def test_raw_v1_must_exactly_match_trusted_canonical_upgrade(self):
        raw = scalar_v1_source()
        canonical = upgrade_scalar_document(raw)
        self.assertTrue(MODULE._verify_source_upgrade(1, raw, 3, canonical))
        tampered = copy.deepcopy(canonical)
        tampered["module"] += "-tampered"
        with self.assertRaisesRegex(ValueError, "does not match trusted"):
            MODULE._verify_source_upgrade(1, raw, 3, tampered)

    def test_same_version_raw_and_canonical_must_be_identical(self):
        source = v3_memory_source()
        self.assertFalse(MODULE._verify_source_upgrade(3, source, 3, copy.deepcopy(source)))
        changed = copy.deepcopy(source)
        changed["module"] += "-changed"
        with self.assertRaisesRegex(ValueError, "same-version"):
            MODULE._verify_source_upgrade(3, source, 3, changed)

    def test_unknown_source_version_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "typed-corehir.json"
            path.write_text(
                json.dumps({"schema": "arukellt-typed-corehir", "schema_version": 4}),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "unsupported TypedCoreHIR source version"):
                MODULE._source_version(path)

    def test_release_command_passes_raw_and_canonical_sources_to_toolchain_builder(self):
        text = (ROOT / "scripts" / "run" / "proof-required-release.sh").read_text(encoding="utf-8")
        call = text.split("python3 scripts/gen/prepare-proof-release-toolchain-v7.py", 1)[1]
        self.assertIn('--typed-corehir-raw "$RAW_TYPED_COREHIR"', call)
        self.assertIn('--typed-corehir "$CANONICAL_TYPED_COREHIR"', call)
        self.assertIn("write-smt-vcs-v7.py", text)
        self.assertNotIn("normalize-source-contract-profile.py", text)


if __name__ == "__main__":
    unittest.main()
