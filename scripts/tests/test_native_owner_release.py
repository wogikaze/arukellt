#!/usr/bin/env python3
"""Phase 5: post-lower CheckedProgram owner release must stay wired and correct."""

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WRAPPER = ROOT / "scripts/run/arukellt-selfhost.sh"
CHECKED = ROOT / "src/compiler/driver/checked_program.ark"
PIPELINE = ROOT / "src/compiler/driver/pipeline_backend.ark"
CONSTANT_FIXTURE = Path("tests/fixtures/native_cpp/constant_return.ark")


class OwnerReleaseTests(unittest.TestCase):
    def test_release_helper_is_defined(self) -> None:
        text = CHECKED.read_text(encoding="utf-8")
        self.assertIn("fn checked_program_release_post_lower(", text)
        self.assertIn("program.bundle.decls = Vec_new_AstNode()", text)
        self.assertIn("program.bundle.check_result = TypeCheckResult_new()", text)

    def test_pipeline_calls_release_after_lower(self) -> None:
        text = PIPELINE.read_text(encoding="utf-8")
        lower_pos = text.find("lower::lower_checked_program(")
        release_pos = text.find("checked_program_release_post_lower(")
        opt_pos = text.find("mir_opt::optimize_module(")
        self.assertGreater(lower_pos, 0)
        self.assertGreater(release_pos, lower_pos)
        self.assertGreater(opt_pos, release_pos)

    def test_native_cpp_fixture_still_compiles(self) -> None:
        build_root = ROOT / ".build-native-recovery"
        build_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=build_root) as tmp:
            output = Path(tmp) / "constant_return.c"
            env = os.environ.copy()
            env["ARUKELLT_BUILD_DIR"] = str(build_root)
            proc = __import__("subprocess").run(
                [
                    str(WRAPPER),
                    "compile",
                    str(CONSTANT_FIXTURE),
                    "--target",
                    "native-cpp",
                    "-o",
                    str(output.relative_to(ROOT)),
                ],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(
                proc.returncode,
                0,
                f"native-cpp compile failed:\n{proc.stderr}\n{proc.stdout}",
            )
            self.assertTrue(output.is_file())
            self.assertIn("ark_", output.read_text(encoding="utf-8", errors="replace"))


if __name__ == "__main__":
    unittest.main()
