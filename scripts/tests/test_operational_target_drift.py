from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "check" / "check-operational-target-drift.py"
SPEC = importlib.util.spec_from_file_location("op_target_drift", SCRIPT)
assert SPEC and SPEC.loader
op_drift = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = op_drift
SPEC.loader.exec_module(op_drift)


class OperationalTargetDriftTest(unittest.TestCase):
    def test_canonical_names_not_flagged(self) -> None:
        """Canonical target names (wasm32, wasm32-gc, native-cpp, native-llvm) must not be flagged."""
        for name in ["wasm32", "wasm32-gc", "native-cpp", "native-llvm"]:
            for pattern in op_drift.DEPRECATED_PATTERNS:
                self.assertIsNone(
                    pattern.search(name),
                    f"Canonical name '{name}' should not match deprecated pattern",
                )

    def test_deprecated_names_flagged(self) -> None:
        """Deprecated target names must be flagged."""
        for name in op_drift._DEPRECATED:
            matched = any(p.search(name) for p in op_drift.DEPRECATED_PATTERNS)
            self.assertTrue(matched, f"Deprecated name '{name}' should match a pattern")

    def test_allowed_dirs(self) -> None:
        """Allowed directories should be skipped."""
        self.assertTrue(op_drift.is_allowed("docs/adr/ADR-007.md"))
        self.assertTrue(op_drift.is_allowed("docs/history/milestones.md"))
        self.assertTrue(op_drift.is_allowed("issues/open/799-test.md"))
        self.assertTrue(op_drift.is_allowed("tmp/test.md"))

    def test_operational_dirs_not_allowed(self) -> None:
        """Operational directories should not be skipped."""
        self.assertFalse(op_drift.is_allowed("scripts/check/check-t3.py"))
        self.assertFalse(op_drift.is_allowed("tests/component-interop/run.sh"))
        self.assertFalse(op_drift.is_allowed("benchmarks/size-compare.sh"))

    def test_allowed_files(self) -> None:
        """Allowed files should be skipped."""
        self.assertTrue(op_drift.is_allowed("scripts/tests/test_target_contract.py"))
        self.assertTrue(op_drift.is_allowed("scripts/check/check-docs-consistency.py"))


if __name__ == "__main__":
    unittest.main()
