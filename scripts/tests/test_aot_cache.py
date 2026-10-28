"""Regression tests for the selfhost AOT cache key.

The wrapper and the Python gates share one ``<cwasm>.tag`` sidecar, so both
must agree on the key and both must reject an image built from other bytes.

Run from the repo root:
    python3 -m unittest scripts.tests.test_aot_cache
"""
from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest.mock import patch

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
if str(REPO_ROOT / "scripts") not in sys.path:
    sys.path.insert(0, str(REPO_ROOT / "scripts"))

from selfhost import checks  # noqa: E402

WRAPPER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"

STUB_WASMTIME = textwrap.dedent(
    """\
    #!/usr/bin/env bash
    set -euo pipefail
    printf '%s\\n' "$*" >> "$STUB_LOG"
    if [[ "${1:-}" == "compile" ]]; then
      output=""
      previous=""
      for arg in "$@"; do
        if [[ "$previous" == "-o" ]]; then
          output="$arg"
        fi
        previous="$arg"
      done
      if [[ -z "$output" ]]; then
        echo "stub wasmtime: compile without -o" >&2
        exit 2
      fi
      printf 'precompiled image' > "$output"
    fi
    exit 0
    """
)


def _bump_mtime(path: Path, seconds: int = 2) -> None:
    """Move mtime forward so the stat identity changes deterministically."""
    info = path.stat()
    os.utime(path, ns=(info.st_atime_ns, info.st_mtime_ns + seconds * 1_000_000_000))


class TestAotCacheKey(unittest.TestCase):
    """The key covers every input that changes the compiled image."""

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.wasm = self.root / "compiler.wasm"
        self.wasm.write_bytes(b"AAAA")
        self.wasmtime = self.root / "wasmtime"
        self.wasmtime.write_bytes(b"binary-v1")
        self.wasmtime.chmod(0o755)

    def test_key_names_artifact_wasmtime_and_collector(self) -> None:
        key = checks._aot_cache_key(self.wasm, str(self.wasmtime))
        self.assertEqual(
            key,
            "aot-cache-v2;collector=copying"
            f";wt={checks._file_identity(self.wasmtime)}"
            f";wasm={checks._file_identity(self.wasm)}",
        )

    def test_rebuilt_wasm_changes_the_key(self) -> None:
        before = checks._aot_cache_key(self.wasm, str(self.wasmtime))
        self.wasm.write_bytes(b"BBBB")  # same size, different compiler
        _bump_mtime(self.wasm)
        self.assertNotEqual(before, checks._aot_cache_key(self.wasm, str(self.wasmtime)))

    def test_touching_the_wasm_changes_the_key(self) -> None:
        before = checks._aot_cache_key(self.wasm, str(self.wasmtime))
        _bump_mtime(self.wasm)
        self.assertNotEqual(before, checks._aot_cache_key(self.wasm, str(self.wasmtime)))

    def test_wasmtime_upgrade_changes_the_key(self) -> None:
        before = checks._aot_cache_key(self.wasm, str(self.wasmtime))
        self.wasmtime.write_bytes(b"binary-v2-longer")
        self.assertNotEqual(before, checks._aot_cache_key(self.wasm, str(self.wasmtime)))


class TestEnsureAotCwasm(unittest.TestCase):
    """A stale or foreign sidecar must force a fresh ``wasmtime compile``."""

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.wasm = self.root / "compiler.wasm"
        self.wasm.write_bytes(b"AAAA")
        self.wasmtime = self.root / "wasmtime"
        self.wasmtime.write_bytes(b"#!/bin/sh\n")
        self.wasmtime.chmod(0o755)
        self.cwasm = self.root / "compiler.collector-copying.cwasm"
        self.tag = self.cwasm.with_suffix(".cwasm.tag")
        self.calls: list[list[str]] = []

    def _compile(self):
        def run(cmd, **_kwargs):
            self.calls.append(list(cmd))
            Path(cmd[cmd.index("-o") + 1]).write_bytes(b"image")
            return subprocess.CompletedProcess(cmd, 0, "", "")

        return run

    def _ensure(self) -> Path:
        with patch.object(checks, "_find_wasmtime", return_value=str(self.wasmtime)):
            with patch.object(checks.subprocess, "run", side_effect=self._compile()):
                return checks._ensure_aot_cwasm(self.wasm)

    def test_second_call_reuses_the_image(self) -> None:
        self.assertEqual(self._ensure(), self.cwasm)
        self.assertEqual(self.tag.read_text(encoding="utf-8").strip(),
                         checks._aot_cache_key(self.wasm, str(self.wasmtime)))
        self.assertEqual(self._ensure(), self.cwasm)
        self.assertEqual(len(self.calls), 1)

    def test_rebuilt_wasm_forces_a_recompile(self) -> None:
        self._ensure()
        self.wasm.write_bytes(b"BBBB")  # same size, different compiler
        _bump_mtime(self.wasm)
        self._ensure()
        self.assertEqual(len(self.calls), 2)
        self.assertEqual(self.tag.read_text(encoding="utf-8").strip(),
                         checks._aot_cache_key(self.wasm, str(self.wasmtime)))

    def test_legacy_sidecar_is_not_trusted(self) -> None:
        self.cwasm.write_bytes(b"stale image")
        self.tag.write_text("collector=copying", encoding="utf-8")
        self._ensure()
        self.assertEqual(len(self.calls), 1)
        self.assertNotEqual(self.cwasm.read_bytes(), b"stale image")

    def test_missing_wasmtime_falls_back_to_the_wasm(self) -> None:
        with patch.object(checks, "_find_wasmtime", return_value=None):
            self.assertEqual(checks._ensure_aot_cwasm(self.wasm), self.wasm)
        self.assertFalse(self.cwasm.exists())


@unittest.skipUnless(sys.platform.startswith("linux"), "wrapper cache uses GNU stat/flock")
class TestWrapperAotCache(unittest.TestCase):
    """The bash wrapper writes the same key and invalidates it on a rewrite."""

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.log = self.root / "wasmtime.log"
        self.stub = self.root / "wasmtime"
        self.stub.write_text(STUB_WASMTIME, encoding="utf-8")
        self.stub.chmod(0o755)
        self.wasm = self.root / "bootstrap" / "arukellt-selfhost.wasm"
        self.wasm.parent.mkdir(parents=True)
        self.wasm.write_bytes(b"pin-A")
        script = self.root / "scripts" / "run" / "arukellt-selfhost.sh"
        script.parent.mkdir(parents=True)
        script.write_text(WRAPPER.read_text(encoding="utf-8"), encoding="utf-8")
        self.cwasm = self.wasm.with_suffix(".collector-copying.cwasm")
        self.tag = self.cwasm.with_suffix(".cwasm.tag")
        self.env = {
            "PATH": "/usr/bin:/bin",
            "HOME": str(self.root / "home"),
            "ARUKELLT_WASMTIME_BIN": str(self.stub),
            "STUB_LOG": str(self.log),
        }

    def _seed_abi_cache(self) -> None:
        info = self.wasm.stat()
        cache = (self.root / ".build" / "selfhost" / "abi"
                 / f"{self.wasm.name}.{info.st_size}-{int(info.st_mtime)}.abi")
        cache.parent.mkdir(parents=True, exist_ok=True)
        cache.write_text("core-p1\n", encoding="utf-8")

    def _run(self) -> None:
        result = subprocess.run(
            ["bash", str(self.root / "scripts" / "run" / "arukellt-selfhost.sh"), "--version"],
            cwd=self.root,
            env=self.env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def _compiles(self) -> int:
        if not self.log.is_file():
            return 0
        return sum(1 for line in self.log.read_text(encoding="utf-8").splitlines()
                   if line.startswith("compile"))

    def test_wrapper_reuses_then_invalidates_the_image(self) -> None:
        self._seed_abi_cache()
        self._run()
        self.assertEqual(self._compiles(), 1)
        self.assertIn("run --allow-precompiled", self.log.read_text(encoding="utf-8"))
        self.assertEqual(self.tag.read_text(encoding="utf-8").strip(),
                         checks._aot_cache_key(self.wasm, str(self.stub)))

        self._run()
        self.assertEqual(self._compiles(), 1, "unchanged wasm must reuse the image")

        self.wasm.write_bytes(b"pin-B")  # same size, rebuilt compiler
        _bump_mtime(self.wasm)
        self._seed_abi_cache()
        self._run()
        self.assertEqual(self._compiles(), 2, "rebuilt wasm must recompile the image")
        self.assertEqual(self.tag.read_text(encoding="utf-8").strip(),
                         checks._aot_cache_key(self.wasm, str(self.stub)))

        _bump_mtime(self.wasm)  # touch only
        self._seed_abi_cache()
        self._run()
        self.assertEqual(self._compiles(), 3, "touched wasm must recompile the image")


if __name__ == "__main__":
    unittest.main()
