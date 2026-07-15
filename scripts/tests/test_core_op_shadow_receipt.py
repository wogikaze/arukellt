#!/usr/bin/env python3
"""Tests for the #798 CoreOp shadow receipt parser."""
from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "check" / "check-core-op-shadow.py"
SPEC = importlib.util.spec_from_file_location("check_core_op_shadow", SCRIPT)
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class CoreOpShadowReceiptTests(unittest.TestCase):
    def test_parse_last_summary(self) -> None:
        stderr = "\n".join(
            [
                "core-op-shadow: summary candidates=2 matched=2 mismatched=0 unresolved=0",
                "core-op-shadow: summary candidates=5 matched=4 mismatched=1 unresolved=0",
            ]
        )
        self.assertEqual(
            module.parse_summary(stderr),
            {"candidates": 5, "matched": 4, "mismatched": 1, "unresolved": 0},
        )

    def test_missing_summary(self) -> None:
        self.assertIsNone(module.parse_summary("ordinary diagnostic"))


if __name__ == "__main__":
    unittest.main()
