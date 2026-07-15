#!/usr/bin/env python3
"""Regression tests for the frozen CoreOp migration alias boundary."""
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


class AliasConsolidationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.alias_map = generator.collect_bindings()

    def test_starts_with_aliases_share_core_op(self) -> None:
        self.assertEqual(self.alias_map["starts_with"], "string.starts_with")
        self.assertEqual(self.alias_map["text::starts_with"], "string.starts_with")

    def test_println_aliases_share_core_op(self) -> None:
        self.assertEqual(self.alias_map["println"], "runtime.println")
        self.assertEqual(self.alias_map["stdio::println"], "runtime.println")

    def test_process_exit_aliases_share_core_op(self) -> None:
        for alias in ("process::exit", "host::process::exit", "__intrinsic_process_exit"):
            self.assertEqual(self.alias_map[alias], "runtime.process.exit")

    def test_process_abort_aliases_share_core_op(self) -> None:
        for alias in ("process::abort", "host::process::abort", "__intrinsic_process_abort"):
            self.assertEqual(self.alias_map[alias], "runtime.process.abort")

    def test_string_slice_aliases_share_core_op(self) -> None:
        for alias in ("slice", "substring", "text::slice_bytes", "__intrinsic_string_slice"):
            self.assertEqual(self.alias_map[alias], "string.slice")

    def test_aliases_are_consolidated_onto_fewer_core_ops(self) -> None:
        self.assertGreater(len(self.alias_map), len(set(self.alias_map.values())))
        self.assertGreaterEqual(len(self.alias_map), 400)


if __name__ == "__main__":
    unittest.main()
