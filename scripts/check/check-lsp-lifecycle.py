#!/usr/bin/env python3
"""Selfhost LSP lifecycle gate (issue #569).

Replays a Content-Length-framed JSON-RPC script through the selfhost
``arukellt lsp`` subcommand and golden-diffs the framed response stream.

The script exercises the full lifecycle:

    initialize → initialized
    textDocument/didOpen   (broken source → ≥1 publishDiagnostics)
    textDocument/didChange (clean source → empty publishDiagnostics)
    shutdown → exit

This is the end-to-end exercise of the LSP lifecycle introduced in #569
on top of the analysis API from #568.
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
            print("error: failed to build s2 wasm from pinned", file=sys.stderr)
            print(r.stderr[:1000], file=sys.stderr)
            return 1

    scripts = sorted(fixtures_dir.glob("lsp_*.lsp-script"))
    if not scripts:
        print("error: no lsp_*.lsp-script fixtures found", file=sys.stderr)
        return 1

    failures = 0
    for script in scripts:
        golden = script.with_suffix(".lsp-expected")
        if not golden.is_file():
            print(f"error: missing golden {golden.relative_to(root)}",
                  file=sys.stderr)
            failures += 1
            continue
        rel = str(script.relative_to(root))
        r = subprocess.run(
            [wasmtime, "run", "--dir", str(root), str(s2), "--",
             "lsp", rel],
            cwd=str(root),
            capture_output=True,
            timeout=60,
        )
        if r.returncode != 0:
            print(f"FAIL {rel}: lsp exit {r.returncode}", file=sys.stderr)
            print(r.stderr[:500].decode("utf-8", errors="replace"),
                  file=sys.stderr)
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
        # Sanity: at least one publishDiagnostics frame must be present.
        if b"publishDiagnostics" not in r.stdout:
            print(f"FAIL {rel}: no publishDiagnostics frame emitted",
                  file=sys.stderr)
            failures += 1
            continue
        print(f"  pass: {rel}")

    if failures > 0:
        print(f"lsp-lifecycle: {failures} failure(s)", file=sys.stderr)
        return 1

    print(f"lsp-lifecycle: {len(scripts)} script(s) pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
