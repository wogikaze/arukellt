#!/usr/bin/env python3
"""Regression tests for CoreOp alias consolidation and handler-key bridge."""
from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "gen"))

from core_op_mapping_common import (  # noqa: E402
    alias_to_core_op_map,
    core_op_to_handler_map,
    extract_handler_branches,
)


class AliasConsolidationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.branches = extract_handler_branches(ROOT / "src" / "compiler" / "wasm")
        cls.alias_map = alias_to_core_op_map(cls.branches)
        cls.handler_map = core_op_to_handler_map(cls.branches)

    def test_starts_with_aliases_share_core_op(self) -> None:
        self.assertEqual(self.alias_map["starts_with"], "string.starts_with")
        self.assertEqual(self.alias_map["text::starts_with"], "string.starts_with")

    def test_println_aliases_share_core_op(self) -> None:
        self.assertEqual(self.alias_map["println"], "runtime.println")
        self.assertEqual(self.alias_map["stdio::println"], "runtime.println")

    def test_process_exit_aliases_share_core_op_and_handler(self) -> None:
        self.assertEqual(self.alias_map["process::exit"], "runtime.process.exit")
        self.assertEqual(self.alias_map["host::process::exit"], "runtime.process.exit")
        self.assertEqual(self.alias_map["__intrinsic_process_exit"], "runtime.process.exit")
        handler = self.handler_map["runtime.process.exit"]
        self.assertIn(
            handler,
            {
                "process::exit",
                "host::process::exit",
                "__intrinsic_process_exit",
            },
        )

    def test_process_abort_handler_recognizable(self) -> None:
        self.assertEqual(self.alias_map["process::abort"], "runtime.process.abort")
        handler = self.handler_map["runtime.process.abort"]
        self.assertIn(
            handler,
            {
                "process::abort",
                "host::process::abort",
                "__intrinsic_process_abort",
            },
        )

    def test_string_slice_aliases_share_core_op_and_handler(self) -> None:
        self.assertEqual(self.alias_map["slice"], "string.slice")
        self.assertEqual(self.alias_map["substring"], "string.slice")
        self.assertEqual(self.alias_map["text::slice_bytes"], "string.slice")
        self.assertEqual(self.alias_map["__intrinsic_string_slice"], "string.slice")
        handler = self.handler_map["string.slice"]
        self.assertIn(
            handler,
            {
                "slice",
                "substring",
                "text::slice_bytes",
                "__intrinsic_string_slice",
            },
        )

    def test_branch_count_much_smaller_than_legacy_literals(self) -> None:
        alias_count = sum(len(b.aliases) for b in self.branches)
        self.assertGreater(alias_count, len(self.branches))
        # Semantic consolidation: branches should be well below 1:1 with ~390 keys.
        self.assertLessEqual(len(self.branches), 320)


if __name__ == "__main__":
    unittest.main()
