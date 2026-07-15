#!/usr/bin/env python3
"""Regression tests for #798 CoreOp migration checkers."""
from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def run(cmd: list[str]) -> int:
    return subprocess.run(cmd, cwd=ROOT).returncode


class CoreOpMigrationChecks(unittest.TestCase):
    def test_core_ops_checker(self) -> None:
        self.assertEqual(run([sys.executable, "scripts/check/check-core-ops.py"]), 0)

    def test_dispatch_freeze(self) -> None:
        self.assertEqual(run([sys.executable, "scripts/check/check-no-new-callee-string-dispatch.py"]), 0)

    def test_legacy_inventory(self) -> None:
        self.assertEqual(run([sys.executable, "scripts/check/check-legacy-dispatch-inventory.py"]), 0)

    def test_registry_generated_fresh(self) -> None:
        self.assertEqual(run([sys.executable, "scripts/gen/generate-core-ops-registry.py", "--check"]), 0)

    def test_bindings_generated_fresh(self) -> None:
        self.assertEqual(run([sys.executable, "scripts/gen/generate-core-op-bindings.py", "--check"]), 0)

    def test_compiler_validator(self) -> None:
        self.assertEqual(run([sys.executable, "scripts/check/check-core-op-compiler-validator.py"]), 0)


if __name__ == "__main__":
    unittest.main()
