#!/usr/bin/env python3
"""Compile and run the GC array smoke fixture as an official P2 component."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path("tests/fixtures/t3/array_gc.ark")
EXPECTED = Path("tests/fixtures/t3/array_gc.expected")
BUILD_DIR = REPO_ROOT / ".build" / "gc-smoke"
OUT_COMPONENT = BUILD_DIR / "array_gc.component.wasm"


def _selfhost_compiler() -> Path | None:
    for candidate in (
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s2-runtime.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s3.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s2.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-pinned-bootstrap.wasm",
        REPO_ROOT / "bootstrap" / "arukellt-selfhost.wasm",
    ):
        if candidate.is_file():
            return candidate
    return None


def main() -> int:
    wrapper = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    wasmtime = os.environ.get("WASMTIME_BIN", "wasmtime")
    if not wrapper.is_file() or not os.access(wrapper, os.X_OK):
        print("SKIP: selfhost wrapper missing", file=sys.stderr)
        return 0
    if subprocess.run([wasmtime, "--version"], capture_output=True).returncode != 0:
        print("SKIP: wasmtime missing", file=sys.stderr)
        return 0

    BUILD_DIR.mkdir(parents=True, exist_ok=True)
    compiler = _selfhost_compiler()
    env = dict(os.environ)
    if compiler is not None:
        env["ARUKELLT_SELFHOST_WASM"] = str(compiler)

    result = subprocess.run(
        [
            str(wrapper),
            "compile",
            "--target",
            "wasm32-gc",
            "--wasi-version",
            "wasi-p2",
            "--emit",
            "component",
            str(FIXTURE),
            "-o",
            str(OUT_COMPONENT.relative_to(REPO_ROOT)),
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=300,
        env=env,
    )
    if result.returncode != 0:
        print("FAIL: compile array_gc component", file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        return 1
    if not OUT_COMPONENT.is_file():
        print(f"FAIL: component not found at {OUT_COMPONENT}", file=sys.stderr)
        return 1

    result = subprocess.run(
        [
            wasmtime,
            "run",
            "--wasm",
            "gc",
            "--wasm",
            "function-references",
            f"--dir={REPO_ROOT}",
            str(OUT_COMPONENT),
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        print("FAIL: direct Wasmtime execution", file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        return 1

    expected = EXPECTED.read_text(encoding="utf-8").strip()
    actual = result.stdout.strip()
    if actual != expected:
        print(f"FAIL: output mismatch: expected {expected!r}, got {actual!r}", file=sys.stderr)
        return 1

    print("check-gc-array-smoke: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
