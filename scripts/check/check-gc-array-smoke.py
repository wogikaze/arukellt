#!/usr/bin/env python3
"""GC array smoke gate: compile + run array_gc fixture with --target wasm32-wasi-p2"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path("tests/fixtures/t3/array_gc.ark")
EXPECTED = Path("tests/fixtures/t3/array_gc.expected")
BUILD_DIR = REPO_ROOT / ".build" / "gc-smoke"
OUT_WASM = BUILD_DIR / "array_gc.wasm"
HOST_RUN = REPO_ROOT / "target" / "release" / "arukellt-host-run"


def main() -> int:
    BUILD_DIR.mkdir(parents=True, exist_ok=True)

    # 1. Resolve selfhost compiler
    wrapper = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
    s2 = BUILD_DIR / "arukellt-s2.wasm"

    # Prefer existing s2 from .build
    s2_candidates = [
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s2.wasm",
        REPO_ROOT / ".build" / "selfhost" / "arukellt-s3.wasm",
    ]
    compiler_wasm = None
    for c in s2_candidates:
        if c.is_file():
            compiler_wasm = c
            break

    if compiler_wasm is None:
        # Build s2 from bootstrap
        bootstrap = REPO_ROOT / "bootstrap" / "arukellt-selfhost.wasm"
        if not bootstrap.is_file():
            print("FAIL: no bootstrap compiler wasm", file=sys.stderr)
            return 1
        r = subprocess.run(
            [str(wrapper), "compile", "src/compiler/main.ark", "-o", str(s2)],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=600,
            env={"ARUKELLT_SELFHOST_WASM": str(bootstrap), **dict(**{"RUST_LOG": ""}, **{k: v for k, v in dict(**os.environ).items()})},
        )
        if r.returncode != 0:
            print("FAIL: s2 build failed", file=sys.stderr)
            print(r.stderr, file=sys.stderr)
            return 1
        compiler_wasm = s2
    else:
        compiler_wasm = compiler_wasm.relative_to(REPO_ROOT)

    # 2. Compile fixture with GC target
    out_rel = OUT_WASM.relative_to(REPO_ROOT)
    r = subprocess.run(
        [str(wrapper), "compile", "--target", "wasm32-wasi-p2", str(FIXTURE), "-o", str(out_rel)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=300,
        env={"ARUKELLT_SELFHOST_WASM": str(compiler_wasm), **dict(**os.environ)},
    )
    if r.returncode != 0:
        print("FAIL: compile array_gc", file=sys.stderr)
        print(r.stderr, file=sys.stderr)
        return 1

    if not OUT_WASM.is_file():
        print(f"FAIL: wasm not found at {OUT_WASM}", file=sys.stderr)
        return 1

    # 3. Run through host-linker (provides P2 import stubs, validates at instantiation)
    if not HOST_RUN.is_file():
        print("FAIL: host-linker not built (run cargo build --release in tools/host-linker)", file=sys.stderr)
        return 1

    r = subprocess.run(
        [str(HOST_RUN), str(OUT_WASM)],
        capture_output=True, text=True, timeout=60,
    )
    if r.returncode != 0:
        print("FAIL: execution", file=sys.stderr)
        print(r.stderr, file=sys.stderr)
        return 1

    # 5. Compare output
    expected = EXPECTED.read_text(encoding="utf-8").strip()
    actual = r.stdout.strip()
    if actual != expected:
        print(f"FAIL: output mismatch", file=sys.stderr)
        print(f"  expected: {expected!r}", file=sys.stderr)
        print(f"  actual:   {actual!r}", file=sys.stderr)
        return 1

    print("check-gc-array-smoke: ok")
    return 0


if __name__ == "__main__":
    import os
    sys.exit(main())
