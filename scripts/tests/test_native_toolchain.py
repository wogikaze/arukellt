"""Unit tests for shared native-cpp clang discovery."""

from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from native.toolchain import MINIMUM_CLANG_VERSION, clang_candidates, resolve_clang


class NativeToolchainTest(unittest.TestCase):
    def test_candidates_prefer_arukellt_cc(self) -> None:
        with mock.patch.dict(os.environ, {"ARUKELLT_CC": "/opt/clang-custom"}, clear=False):
            self.assertEqual(clang_candidates(), ["/opt/clang-custom"])

    def test_candidates_default_order(self) -> None:
        with mock.patch.dict(os.environ, {}, clear=False):
            os.environ.pop("ARUKELLT_CC", None)
            self.assertEqual(
                clang_candidates(),
                ["clang", "clang-18", "clang-17", "clang-16", "clang-15", "clang-14"],
            )

    def test_reject_gcc_masquerading_as_override(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fake = Path(temp_dir) / "gcc"
            fake.write_text("#!/bin/sh\necho 'gcc (Ubuntu) 11.4.0'\n", encoding="utf-8")
            fake.chmod(0o755)
            with mock.patch.dict(os.environ, {"ARUKELLT_CC": str(fake)}, clear=False):
                toolchain, diagnostic = resolve_clang()
            self.assertIsNone(toolchain)
            self.assertIn("clang", diagnostic.lower())

    def test_accept_real_clang_when_available(self) -> None:
        toolchain, diagnostic = resolve_clang()
        if toolchain is None:
            self.skipTest(diagnostic)
        self.assertGreaterEqual(toolchain.major_version, MINIMUM_CLANG_VERSION)
        self.assertTrue(Path(toolchain.path).is_file())
        self.assertEqual(diagnostic, "")


if __name__ == "__main__":
    unittest.main()
