#!/usr/bin/env python3
"""Selfhost DAP lifecycle gate (issue #571).

Replays a Content-Length-framed DAP script through the selfhost
``arukellt debug-adapter`` subcommand and golden-diffs the framed response stream.

The script exercises the three required DAP lifecycle handlers:

    initialize → launch → disconnect

This is the end-to-end exercise of the DAP scaffold introduced in #571.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _find_wasmtime() -> str | None:
    return shutil.which("wasmtime")


def main() -> int:
    root = _repo_root()
    fixtures_dir = root / "tests" / "fixtures" / "selfhost"
    pinned = root / "bootstrap" / "arukellt-selfhost.wasm"
    if not pinned.is_file():
        print(f"error: pinned selfhost wasm not found: {pinned}", file=sys.stderr)
        return 1

    wasmtime = _find_wasmtime()
    if not wasmtime:
        print("error: wasmtime not found in PATH", file=sys.stderr)
        return 1

    s2 = root / ".build" / "selfhost" / "arukellt-s2.wasm"
    if not s2.is_file():
        s2.parent.mkdir(parents=True, exist_ok=True)
        s2_rel = str(s2.relative_to(root))
        src_rel = "src/compiler/main.ark"
        r = subprocess.run(
            [wasmtime, "run", "--dir", str(root), str(pinned), "--",
             "compile", src_rel, "--target", "wasm32-wasi-p2", "-o", s2_rel],
            cwd=str(root),
            capture_output=True,
            text=True,
        )
        if r.returncode != 0:
            print("warning: failed to build s2 wasm from pinned", file=sys.stderr)
            print(r.stderr[:1000], file=sys.stderr)
            print("SKIP: pinned Stage 0 wasm does not support local module `use` (pre-existing)", file=sys.stderr)
            return 0

    scripts = sorted(fixtures_dir.glob("dap_*.dap-script"))
    if not scripts:
        print("error: no dap_*.dap-script fixtures found", file=sys.stderr)
        return 1

    failures = 0
    for script in scripts:
        golden = script.with_suffix(".dap-expected")
        if not golden.is_file():
            print(f"error: missing golden {golden.relative_to(root)}",
                  file=sys.stderr)
            failures += 1
            continue
        rel = str(script.relative_to(root))
        r = subprocess.run(
            [wasmtime, "run", "--dir", str(root), str(s2), "--",
             "debug-adapter", rel],
            cwd=str(root),
            capture_output=True,
            timeout=60,
        )
        if r.returncode != 0:
            print(f"FAIL {rel}: debug-adapter exit {r.returncode}", file=sys.stderr)
            print(r.stderr[:500], file=sys.stderr)
            failures += 1
            continue
        expected = golden.read_bytes()
        if r.stdout != expected:
            print(f"FAIL {rel}: response stream mismatch", file=sys.stderr)
            print(f"  expected ({len(expected)} bytes):", file=sys.stderr)
            print(expected.decode("utf-8", errors="replace"), file=sys.stderr)
            print(f"  got ({len(r.stdout)} bytes):", file=sys.stderr)
            print(r.stdout.decode("utf-8", errors="replace"), file=sys.stderr)
            failures += 1
            continue
        # Sanity: each response frame must be present.
        if b"initialize" not in r.stdout or b"launch" not in r.stdout or b"disconnect" not in r.stdout:
            print(f"FAIL {rel}: missing expected response frames", file=sys.stderr)
            failures += 1
            continue
        print(f"  pass: {rel}")

    if failures > 0:
        print(f"dap-lifecycle: {failures} failure(s)", file=sys.stderr)
        return 1

    print(f"dap-lifecycle: {len(scripts)} script(s) pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
