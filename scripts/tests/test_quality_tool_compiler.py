"""Tests for the selfhost compiler used by quality fmt/lint subprocesses.

The lane fmt/lint path must resolve the compiler from the effective build dir
(``ARUKELLT_BUILD_DIR`` or ``<root>/.build``) instead of hard-coding
``<root>/.build``, otherwise an isolated worktree silently lints with the
pinned bootstrap compiler.
"""

from __future__ import annotations

import contextlib
import io
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts.quality import checks


def _wasm(path: Path, payload: bytes = b"\0asm") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload)
    return path


def _completed_run(*_args, **_kwargs) -> subprocess.CompletedProcess:
    return subprocess.CompletedProcess((), 0, "", "")


class TestToolCompilerResolution(unittest.TestCase):
    def test_build_dir_compiler_wins_over_worktree_dot_build(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "worktree"
            lane_build = Path(tmp) / "lane-build"
            _wasm(root / ".build/selfhost/arukellt-s2-runtime.wasm")
            lane = _wasm(lane_build / "selfhost/arukellt-s2-runtime.wasm")
            with mock.patch.dict(os.environ, {"ARUKELLT_BUILD_DIR": str(lane_build)}):
                self.assertEqual(checks._tool_compiler_wasm(root), lane.resolve())

    def test_build_dir_compiler_prefers_runtime_over_plain_s2(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "worktree"
            _wasm(root / ".build/selfhost/arukellt-s2.wasm")
            runtime = _wasm(root / ".build/selfhost/arukellt-s2-runtime.wasm")
            with mock.patch.dict(os.environ, {}, clear=True):
                self.assertEqual(checks._tool_compiler_wasm(root), runtime.resolve())

    def test_build_dir_compiler_falls_back_to_plain_s2(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "worktree"
            lane_build = Path(tmp) / "lane-build"
            lane = _wasm(lane_build / "selfhost/arukellt-s2.wasm")
            with mock.patch.dict(os.environ, {"ARUKELLT_BUILD_DIR": str(lane_build)}):
                self.assertEqual(checks._tool_compiler_wasm(root), lane.resolve())

    def test_missing_compiler_resolves_to_none(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "worktree"
            root.mkdir()
            with mock.patch.dict(os.environ, {}, clear=True):
                self.assertIsNone(checks._tool_compiler_wasm(root))


class TestFormatterCacheIdentity(unittest.TestCase):
    def test_lane_compiler_defines_the_formatter_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "worktree"
            lane_build = Path(tmp) / "lane-build"
            _wasm(lane_build / "selfhost/arukellt-s2-runtime.wasm", b"\0asm-lane")
            _wasm(root / "bootstrap/arukellt-selfhost.wasm", b"\0asm-pinned")
            with mock.patch.dict(
                os.environ, {"ARUKELLT_BUILD_DIR": str(lane_build)}, clear=True
            ):
                self.assertTrue(
                    checks._formatter_tool_id(root).startswith("arukellt-s2-runtime.wasm:")
                )


class TestRunToolCompilerEnvironment(unittest.TestCase):
    def _run_and_capture(self, root: Path, env: dict) -> tuple[dict, str]:
        stderr = io.StringIO()
        with mock.patch.dict(os.environ, env, clear=True), mock.patch.object(
            checks, "_tool_compiler_notice_shown", False
        ), mock.patch.object(
            checks.subprocess, "run", side_effect=_completed_run
        ) as run_mock, contextlib.redirect_stderr(stderr):
            checks._run_tool(root, "sample.ark", ("tool", "lint"), dry_run=False)
        return run_mock.call_args.kwargs["env"], stderr.getvalue()

    def test_tool_subprocess_receives_the_lane_compiler(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "worktree"
            lane_build = Path(tmp) / "lane-build"
            lane = _wasm(lane_build / "selfhost/arukellt-s2-runtime.wasm")
            env, _note = self._run_and_capture(
                root, {"ARUKELLT_BUILD_DIR": str(lane_build)}
            )
            self.assertEqual(env["ARUKELLT_SELFHOST_WASM"], str(lane.resolve()))

    def test_explicit_selfhost_wasm_override_is_preserved(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "worktree"
            lane_build = Path(tmp) / "lane-build"
            _wasm(lane_build / "selfhost/arukellt-s2-runtime.wasm")
            env, _note = self._run_and_capture(
                root,
                {
                    "ARUKELLT_BUILD_DIR": str(lane_build),
                    "ARUKELLT_SELFHOST_WASM": "/tmp/explicit-compiler.wasm",
                },
            )
            self.assertEqual(
                env["ARUKELLT_SELFHOST_WASM"], "/tmp/explicit-compiler.wasm"
            )

    def test_missing_lane_compiler_keeps_the_wrapper_fallback(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "worktree"
            lane_build = Path(tmp) / "lane-build"
            env, note = self._run_and_capture(
                root, {"ARUKELLT_BUILD_DIR": str(lane_build)}
            )
            self.assertNotIn("ARUKELLT_SELFHOST_WASM", env)
            self.assertIn("no selfhost compiler", note)
            self.assertIn(str(lane_build.resolve() / "selfhost"), note)

    def test_bootstrap_fallback_note_is_printed_once(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "worktree"
            lane_build = Path(tmp) / "lane-build"
            stderr = io.StringIO()
            with mock.patch.dict(
                os.environ, {"ARUKELLT_BUILD_DIR": str(lane_build)}, clear=True
            ), mock.patch.object(
                checks, "_tool_compiler_notice_shown", False
            ), mock.patch.object(
                checks.subprocess, "run", side_effect=_completed_run
            ), contextlib.redirect_stderr(stderr):
                checks._run_tool(root, "a.ark", ("tool",), dry_run=False)
                checks._run_tool(root, "b.ark", ("tool",), dry_run=False)
            self.assertEqual(stderr.getvalue().count("no selfhost compiler"), 1)


if __name__ == "__main__":
    unittest.main()
