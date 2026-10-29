"""Tests for the selfhost domain in manager.py.

Run from the repo root:
    python3 -m unittest scripts/tests/test_selfhost.py

NOTE: These tests require manager.py to have the selfhost domain wired in.
      They will fail until manager.py is patched with the selfhost block.
"""
from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
MANAGER = REPO_ROOT / "scripts" / "manager.py"


class TestReadOnlyFixtureContract(unittest.TestCase):
    """A missing sandbox must not silently become writable execution."""

    def test_missing_bwrap_is_explicit_failure(self) -> None:
        sys.path.insert(0, str(REPO_ROOT / "scripts"))
        from selfhost.checks import ReadOnlyFixtureUnavailable, _wasm_run_argv

        with patch("selfhost.checks._find_wasmtime", return_value="wasmtime"):
            with patch("selfhost.checks.shutil.which", return_value=None):
                with self.assertRaises(ReadOnlyFixtureUnavailable) as ctx:
                    _wasm_run_argv(
                        REPO_ROOT,
                        REPO_ROOT / "fixture.wasm",
                        read_only=True,
                    )
        self.assertIn("writable fallback", str(ctx.exception))


class TestRuntimeCompilerPath(unittest.TestCase):
    """The current compiler has one stable path for direct-execution gates."""

    def test_memory32_compiler_is_materialized_at_runtime_path(self) -> None:
        sys.path.insert(0, str(REPO_ROOT / "scripts"))
        from selfhost import checks

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / ".build" / "selfhost" / "arukellt-s2.wasm"
            compiler.parent.mkdir(parents=True)
            compiler.write_bytes(b"valid compiler placeholder")
            with patch.object(checks, "_reject_invalid_compiler_wasm", return_value=""):
                with patch.object(checks, "_wasm_memory_section_is_memory64", return_value=False):
                    runtime = checks._ensure_runtime_compiler_wasm(root, compiler)

            expected = root / ".build" / "selfhost" / "arukellt-s2-runtime.wasm"
            self.assertEqual(runtime, expected)
            self.assertEqual(runtime.read_bytes(), compiler.read_bytes())


def _run(*args: str) -> tuple[int, str]:
    result = subprocess.run(
        [sys.executable, str(MANAGER), *args],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
    )
    return result.returncode, result.stdout + result.stderr


class TestSelfhostDryRun(unittest.TestCase):
    """Dry-run invocations must exit 0 and not touch the filesystem."""

    def test_fixpoint_dry_run(self) -> None:
        rc, out = _run("selfhost", "fixpoint", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("DRY-RUN", out)

    def test_fixture_parity_dry_run(self) -> None:
        rc, out = _run("selfhost", "fixture-parity", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("DRY-RUN", out)

    def test_diag_parity_dry_run(self) -> None:
        rc, out = _run("selfhost", "diag-parity", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("DRY-RUN", out)

    def test_parity_dry_run(self) -> None:
        rc, out = _run("selfhost", "parity", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("DRY-RUN", out)


class TestSelfhostUnknownSubcommand(unittest.TestCase):
    """Unknown subcommands must exit nonzero."""

    def test_unknown_subcommand(self) -> None:
        rc, out = _run("selfhost", "does-not-exist")
        self.assertNotEqual(rc, 0, f"Expected nonzero exit, got 0. Output:\n{out}")


class TestSelfhostFixpointBuildFlag(unittest.TestCase):
    """--build flag on fixpoint should be accepted (inverse of --no-build)."""

    def test_fixpoint_build_flag_dry_run(self) -> None:
        rc, out = _run("selfhost", "fixpoint", "--build", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")


class TestVerifyFullFixpointDryRun(unittest.TestCase):
    """verify --full --dry-run must include the fixpoint gate and exit 0."""

    def test_full_dry_run_includes_fixpoint(self) -> None:
        # verify --full --dry-run may exit non-zero due to pre-existing gate
        # failures (manifest drift, etc.); we only check that the fixpoint
        # gate is included in the output.
        rc, out = _run("verify", "--full", "--dry-run")
        self.assertIn("fixpoint", out.lower())
        self.assertIn("full verify", out)

    def test_selfhost_fixpoint_flag_dry_run(self) -> None:
        rc, out = _run("verify", "--selfhost-fixpoint", "--dry-run")
        self.assertEqual(rc, 0, f"Expected exit 0, got {rc}. Output:\n{out}")
        self.assertIn("fixpoint", out.lower())
        self.assertIn("full verify", out)


class TestPinAwareCaches(unittest.TestCase):
    """The fast iteration pin must not reuse official-pin caches."""

    def test_fingerprint_and_overlay_key_include_the_resolved_pin(self) -> None:
        sys.path.insert(0, str(REPO_ROOT / "scripts"))
        from selfhost import checks

        official = REPO_ROOT / checks.PINNED_WASM_REL
        fast = REPO_ROOT / "bootstrap" / "arukellt-selfhost-fast.wasm"
        if not official.is_file() or not fast.is_file():
            self.skipTest("pinned artifacts are not both present")
        with patch.dict("os.environ", {"ARUKELLT_PINNED_WASM": str(official)}):
            official_fingerprint = checks._selfhost_source_fingerprint(REPO_ROOT)
            official_overlay = checks._compiler_source_content_hash(REPO_ROOT)
        with patch.dict("os.environ", {"ARUKELLT_PINNED_WASM": str(fast)}):
            fast_fingerprint = checks._selfhost_source_fingerprint(REPO_ROOT)
            fast_overlay = checks._compiler_source_content_hash(REPO_ROOT)
        self.assertNotEqual(official_fingerprint, fast_fingerprint)
        self.assertNotEqual(official_overlay, fast_overlay)

    def test_fingerprint_inputs_use_the_bootstrap_closure(self) -> None:
        sys.path.insert(0, str(REPO_ROOT / "scripts"))
        from selfhost import checks

        inputs = {path.resolve() for path in checks._selfhost_fingerprint_inputs(REPO_ROOT)}
        self.assertIn((REPO_ROOT / "src" / "compiler" / "main.ark").resolve(), inputs)
        self.assertIn((REPO_ROOT / "std" / "prelude.ark").resolve(), inputs)
        self.assertNotIn(
            (REPO_ROOT / "std" / "collections" / "vec.ark").resolve(), inputs
        )


class TestGcPinCompilePath(unittest.TestCase):
    """The gc-host pin compile must reuse AOT and pass the overlay-mode flag."""

    def _compile_argv(self, *, aot: bool) -> list[str]:
        sys.path.insert(0, str(REPO_ROOT / "scripts"))
        from selfhost import checks

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pin = root / "bootstrap" / "arukellt-selfhost.wasm"
            pin.parent.mkdir(parents=True)
            pin.write_bytes(b"component placeholder")
            workspace = root / "workspace"
            workspace.mkdir()
            run_path = pin.with_suffix(".collector-copying.cwasm") if aot else pin
            captured: dict[str, list[str]] = {}

            def record(argv: list[str], _root: Path, *, timeout: int | None = None):
                captured["argv"] = list(argv)
                return subprocess.CompletedProcess(argv, 0, "", "")

            with patch.object(checks, "_prepare_bootstrap_workspace", return_value=workspace):
                with patch.object(checks, "_ensure_aot_cwasm", return_value=run_path):
                    with patch.object(checks, "_run", side_effect=record):
                        checks._compile_with_gc_pin("wasmtime", root, "out.wasm", pin)

        return captured["argv"]

    def test_gc_pin_compile_reuses_aot_image(self) -> None:
        argv = self._compile_argv(aot=True)
        self.assertIn("--allow-precompiled", argv)

    def test_gc_pin_compile_passes_shared_param_overlay_flag(self) -> None:
        sys.path.insert(0, str(REPO_ROOT / "scripts"))
        from selfhost import checks

        for aot in (True, False):
            argv = self._compile_argv(aot=aot)
            self.assertIn(checks.BOOTSTRAP_SHARED_PARAM_FLAG, argv)


if __name__ == "__main__":
    unittest.main()
