#!/usr/bin/env python3
"""Selfhost analysis API gate (issue #568).

Runs ``arukellt ide-analyze`` on every ``tests/fixtures/selfhost/analysis_*.ark``
fixture using the current selfhost wasm and compares stdout against a
committed ``.analysis-expected`` golden file.

This is the end-to-end exercise of the analysis API surface introduced
in #568:

    text → lex → parse → resolve → typecheck → AnalysisSnapshot

The gate fails if any fixture's output diverges from its golden.
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
        # Build s2 from pinned wasm + current selfhost source.
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

    fixtures = sorted(fixtures_dir.glob("analysis_*.ark"))
    if len(fixtures) < 2:
        print(f"error: expected ≥2 analysis fixtures, found {len(fixtures)}",
              file=sys.stderr)
        return 1

    failures = 0
    for fixture in fixtures:
        golden = fixture.with_suffix(".analysis-expected")
        if not golden.is_file():
            print(f"error: missing golden {golden.relative_to(root)}",
                  file=sys.stderr)
            failures += 1
            continue
        rel = str(fixture.relative_to(root))
        r = subprocess.run(
            [wasmtime, "run", "--dir", str(root), str(s2), "--",
             "ide-analyze", rel],
            cwd=str(root),
            capture_output=True,
            text=True,
            timeout=60,
        )
        if r.returncode != 0:
            print(f"FAIL {rel}: ide-analyze exit {r.returncode}",
                  file=sys.stderr)
            print(r.stderr[:500], file=sys.stderr)
            failures += 1
            continue
        expected = golden.read_text()
        if r.stdout != expected:
            print(f"FAIL {rel}: snapshot mismatch", file=sys.stderr)
            print(f"  expected:\n{expected}", file=sys.stderr)
            print(f"  got:\n{r.stdout}", file=sys.stderr)
            failures += 1
            continue
        print(f"  pass: {rel}")

    if failures > 0:
        print(f"analysis-api: {failures} failure(s)", file=sys.stderr)
        return 1

    print(f"analysis-api: {len(fixtures)} fixture(s) pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
