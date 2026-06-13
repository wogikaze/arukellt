#!/usr/bin/env python3
"""CLI vs snapshot parity for fmt --fix (#440)."""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _resolve_compiler_wasm(root: Path) -> Path | None:
    from scripts.selfhost.checks import resolve_ide_gate_compiler_wasm, run_fixpoint

    compiler = resolve_ide_gate_compiler_wasm(root)
    if compiler is not None:
        return compiler
    run_fixpoint(root, dry_run=False, no_build=False)
    return resolve_ide_gate_compiler_wasm(root)


def main() -> int:
    root = _repo_root()
    wasmtime = shutil.which("wasmtime")
    if not wasmtime:
        print("error: wasmtime not found in PATH", file=sys.stderr)
        return 1

    sys.path.insert(0, str(root))
    compiler = _resolve_compiler_wasm(root)
    if compiler is None:
        print("error: no selfhost compiler wasm available", file=sys.stderr)
        return 1

    snap_dir = root / "tests" / "snapshots" / "fix"
    input_path = snap_dir / "organize_imports.input.ark"
    expected_path = snap_dir / "organize_imports.fixed.ark"
    if not input_path.is_file() or not expected_path.is_file():
        print("error: missing fix snapshot fixtures", file=sys.stderr)
        return 1

    expected = expected_path.read_text(encoding="utf-8")
    work = snap_dir / "_fix_pipeline_work.ark"
    try:
        work.write_text(input_path.read_text(encoding="utf-8"), encoding="utf-8")
        r = subprocess.run(
            [wasmtime, "run", "--dir", str(snap_dir), str(compiler), "--",
             "fmt", work.name, "--fix"],
            cwd=str(root),
            capture_output=True,
            text=True,
        )
        if r.returncode != 0:
            print(f"error: fmt --fix failed: {r.stderr}", file=sys.stderr)
            return 1
        got = work.read_text(encoding="utf-8")
        if got != expected:
            print("FAIL: fmt --fix output mismatch", file=sys.stderr)
            print("expected:", file=sys.stderr)
            print(expected, file=sys.stderr)
            print("got:", file=sys.stderr)
            print(got, file=sys.stderr)
            return 1
    finally:
        if work.is_file():
            work.unlink()

    print("check-fix-pipeline: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
