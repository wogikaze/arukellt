#!/usr/bin/env python3
"""Regression tests for the retired CoreOp prefix-family binding path.

`data/core-ops.toml` no longer defines `legacy_binding_patterns`: named-type
`Vec_new_*` constructors were retired with ADR-040, so generated bindings
resolve exact `legacy_bindings` / manifest aliases only.
"""
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


class FamilyBindingRetirementTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.alias_map = generator.collect_bindings()
        cls.rendered = generator.render(cls.alias_map)

    def test_removed_vec_new_family_aliases_are_not_generated(self) -> None:
        for callee in (
            "Vec_new_String",
            "Vec_new_i64",
            "Vec_new_TomlValue",
            "__intrinsic_Vec_new_Cell",
        ):
            self.assertNotIn(callee, self.alias_map)

    def test_exact_aliases_still_resolve_current_map(self) -> None:
        self.assertEqual(self.alias_map["String_from"], "text.clone")
        self.assertEqual(self.alias_map["Vec::len"], "vec.len")

    def test_generated_lookup_is_exact_only(self) -> None:
        self.assertIn("fn core_op_binding_core_op_id_for_callee(", self.rendered)
        self.assertNotIn("core_op_binding_core_op_id_for_callee_family", self.rendered)
        self.assertNotIn(
            "core_op_binding_core_op_id_for_callee_exact_then_family",
            self.rendered,
        )
        self.assertNotIn("core_op_binding_pattern_count", self.rendered)


if __name__ == "__main__":
    unittest.main()
