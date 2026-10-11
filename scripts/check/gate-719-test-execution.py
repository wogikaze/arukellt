#!/usr/bin/env python3
"""Gate for #719 — arukellt test execution harness (ADR-041 Phase 2).

Proves:
1. `arukellt test` on a test-decl fixture executes tests and exits 0 with
   per-test PASS lines (fail-fast runner, wasm32 exec target).
2. A failing test body traps and the harness exits non-zero.
3. A file without test declarations exits 2 with a clear diagnostic.
4. `i32.pow` / `i32.gcd` emit valid Wasm under wasm32-gc (memory64 scalar
   widen) — regression for the raw OP_I32_* emission fixed alongside #719.

Requires a stage-2 selfhost artifact (the pinned bootstrap predates the
execution harness); SKIPs when none is available.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
WRAPPER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"


def _find_compiler() -> Path | None:
    """Locate a stage-2 selfhost artifact that contains the test runner.

    The pinned bootstrap predates `arukellt test` execution, so unlike
    other gates this does not fall back to bootstrap/.
    """
    env = os.environ.get("ARUKELLT_SELFHOST_WASM", "").strip()
    if env:
        path = Path(env)
        if not path.is_absolute():
            path = REPO_ROOT / path
        if path.is_file():
            return path
    for rel in (
        ".build/selfhost/arukellt-s2-runtime.wasm",
        ".build/selfhost/arukellt-s2.wasm",
        ".build/selfhost/arukellt-s3.wasm",
    ):
        path = REPO_ROOT / rel
        if path.is_file():
            return path
    return None


def _run(
    compiler: Path,
    args: list[str],
    timeout: int = 240,
) -> subprocess.CompletedProcess[str]:
    env = dict(os.environ)
    env["ARUKELLT_SELFHOST_WASM"] = str(compiler)
    return subprocess.run(
        ["bash", str(WRAPPER), *args],
        cwd=str(REPO_ROOT),
        env=env,
        capture_output=True,
        text=True,
        timeout=timeout,
    )


def _validate_wasm(path: Path) -> tuple[int, str]:
    wasm_tools = shutil.which("wasm-tools")
    if wasm_tools is None:
        return 0, "wasm-tools not found; skipping validate"
    result = subprocess.run(
        [wasm_tools, "validate", "--features", "gc", str(path)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stderr or result.stdout)[-400:]
    return 0, ""


def main() -> int:
    if shutil.which("wasmtime") is None:
        print("gate-719-test-execution: SKIP (wasmtime not found)")
        return 0
    if not WRAPPER.is_file():
        print("gate-719-test-execution: SKIP (arukellt-selfhost.sh not found)")
        return 0
    compiler = _find_compiler()
    if compiler is None:
        print("gate-719-test-execution: SKIP (no stage-2 selfhost artifact)")
        return 0

    failures: list[str] = []
    out_dir = Path(
        tempfile.mkdtemp(prefix="gate-719-test-", dir=REPO_ROOT / ".build")
    )
    try:
        # 1. All tests pass: exit 0 with per-test PASS lines.
        result = _run(
            compiler,
            ["test", "tests/fixtures/test_syntax/basic.ark"],
        )
        out = (result.stdout or "") + "\n" + (result.stderr or "")
        if result.returncode != 0:
            failures.append(f"basic.ark exit {result.returncode}: {out[-400:]}")
        else:
            for expected in (
                "PASS sanity",
                "PASS adds_two_numbers",
                "PASS normalize suite::idempotent",
                "PASS normalize suite::empty",
            ):
                if expected not in out:
                    failures.append(f"basic.ark missing {expected!r} in output")

        # 2. Failing test traps: non-zero exit, assertion message visible.
        result = _run(
            compiler,
            ["test", "tests/fixtures/test_syntax/assert_fail.ark"],
        )
        out = (result.stdout or "") + "\n" + (result.stderr or "")
        if result.returncode == 0:
            failures.append("assert_fail.ark unexpectedly exited 0")
        elif "assertion failed" not in out:
            failures.append(
                f"assert_fail.ark missing assertion message: {out[-400:]}"
            )

        # 3. No test declarations: exit 2 with diagnostic.
        result = _run(
            compiler,
            ["test", "tests/fixtures/hello/hello.ark"],
        )
        out = (result.stdout or "") + "\n" + (result.stderr or "")
        if result.returncode != 2:
            failures.append(f"hello.ark exit {result.returncode}, expected 2")
        elif "no test declarations" not in out:
            failures.append(
                f"hello.ark missing no-tests diagnostic: {out[-400:]}"
            )

        # 4. pow/gcd under wasm32-gc must emit a valid module (memory64
        # scalar widen turns raw OP_I32_* emission into a type mismatch).
        for name in ("math_pow_basic.ark", "math_gcd_basic.ark"):
            fixture = f"tests/fixtures/stdlib_core/{name}"
            out_wasm = out_dir / f"{name}.wasm"
            result = _run(
                compiler,
                [
                    "compile",
                    fixture,
                    "--target",
                    "wasm32-gc",
                    "-o",
                    str(out_wasm.relative_to(REPO_ROOT)),
                ],
            )
            if result.returncode != 0:
                failures.append(
                    f"{name} gc compile failed: "
                    + ((result.stderr or result.stdout or "")[-400:])
                )
                continue
            if not out_wasm.is_file():
                failures.append(f"{name} gc compile produced no output")
                continue
            rc, msg = _validate_wasm(out_wasm)
            if rc != 0:
                failures.append(f"{name} gc module invalid: {msg}")
    finally:
        shutil.rmtree(out_dir, ignore_errors=True)

    if failures:
        print("gate-719-test-execution: FAIL", file=sys.stderr)
        for line in failures:
            print(f"  - {line}", file=sys.stderr)
        return 1
    print("gate-719-test-execution: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
