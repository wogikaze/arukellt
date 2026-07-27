#!/usr/bin/env python3
"""Tests for CoreOp legacy_binding_patterns family generation."""
from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "scripts" / "gen" / "generate-core-op-bindings.py"
SPEC = importlib.util.spec_from_file_location("generate_core_op_bindings", GENERATOR)
assert SPEC and SPEC.loader
generator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(generator)


class FamilyBindingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.alias_map = generator.collect_bindings()
        cls.patterns = generator.collect_family_patterns()
        cls.rendered = generator.render(cls.alias_map, cls.patterns)

    def test_patterns_are_present_and_longest_first(self) -> None:
        self.assertGreaterEqual(len(self.patterns), 2)
        prefixes = [str(p["pattern"]) for p in self.patterns]
        self.assertIn("Vec_new_", prefixes)
        self.assertIn("__intrinsic_Vec_new_", prefixes)
        # Longer prefix must appear before shorter when one is a prefix of the other.
        lengths = [len(p) for p in prefixes]
        self.assertEqual(lengths, sorted(lengths, reverse=True))

    def test_exact_aliases_still_win_in_map(self) -> None:
        self.assertEqual(self.alias_map["Vec_new_String"], "vec.Vec_new_fallback")
        self.assertEqual(self.alias_map["Vec_new_i64"], "vec.Vec_new_f64")
        self.assertNotIn("Vec_new_TomlValue", self.alias_map)

    def test_family_api_emitted(self) -> None:
        self.assertIn("fn core_op_binding_core_op_id_for_callee_exact(", self.rendered)
        self.assertIn("fn core_op_binding_core_op_id_for_callee_family(", self.rendered)
        self.assertIn(
            "fn core_op_binding_core_op_id_for_callee_exact_then_family(", self.rendered
        )
        # Compatibility exact wrapper remains.
        self.assertIn("fn core_op_binding_core_op_id_for_callee(", self.rendered)
        self.assertIn("core_op_binding_pattern_count()", self.rendered)

    def test_family_lookup_helper_matches_named_type(self) -> None:
        # Mirror generated longest-prefix + nonempty-suffix rules in Python.
        def family(callee: str) -> str:
            for pattern in self.patterns:
                prefix = str(pattern["pattern"])
                if callee.startswith(prefix):
                    suffix = callee[len(prefix) :]
                    if pattern["require_nonempty_suffix"] and not suffix:
                        continue
                    return str(pattern["core_op_id"])
            return ""

        self.assertEqual(family("Vec_new_TomlValue"), "vec.Vec_new_fallback")
        self.assertEqual(family("Vec_new_JsonValue"), "vec.Vec_new_fallback")
        self.assertEqual(family("__intrinsic_Vec_new_Cell"), "vec.Vec_new_fallback")
        self.assertEqual(family("Vec_new_"), "")
        self.assertEqual(family("Vec_push"), "")

    def test_conflicting_patterns_error(self) -> None:
        bad = {
            "operations": [{"id": "vec.Vec_new_fallback"}],
            "legacy_binding_patterns": [
                {
                    "kind": "prefix",
                    "pattern": "Vec_new_",
                    "core_op_id": "vec.Vec_new_fallback",
                    "precedence": "after_exact",
                    "require_nonempty_suffix": True,
                },
                {
                    "kind": "prefix",
                    "pattern": "Vec_new_",
                    "core_op_id": "vec.Vec_new_f64",
                    "precedence": "after_exact",
                    "require_nonempty_suffix": True,
                },
            ],
        }
        with self.assertRaises(ValueError):
            generator.collect_family_patterns(bad)

    def test_unknown_core_op_id_errors(self) -> None:
        bad = {
            "operations": [{"id": "vec.Vec_new_fallback"}],
            "legacy_binding_patterns": [
                {
                    "kind": "prefix",
                    "pattern": "Vec_new_",
                    "core_op_id": "vec.does_not_exist",
                    "precedence": "after_exact",
                    "require_nonempty_suffix": True,
                }
            ],
        }
        with self.assertRaises(ValueError):
            generator.collect_family_patterns(bad)


if __name__ == "__main__":
    unittest.main()
