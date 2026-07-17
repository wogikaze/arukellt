#!/usr/bin/env python3
"""Regression gate for Vec_new u32 wraparound guard in the wasm emitter.

Compiles a set of stdlib_vec fixtures to wasm32, runs them, and verifies that the
generated WAT contains the expected overflow-guard sequence for the linear-memory
bump allocator:

    global.get 0
    ... i32.add (compute new_end = old_result + total_size)
    ... i32.lt_u (new_end < old_result)
    ... select
    ... global.set 0

The guard is emitted inline for every Vec_new call (including constructors with a
capacity argument, where the argument is dropped to keep the stack clean).
"""

import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GATE_DIR = ROOT / ".build" / "gate"
SELFHOST = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"

FIXTURES = [
    "stdlib_vec/vec_new.ark",
    "stdlib_vec/vec_new_i64.ark",
    "stdlib_vec/vec_new_f64.ark",
    "stdlib_vec/vec_new_string.ark",
    "stdlib_vec/vec_new_with_capacity_i32.ark",
    "stdlib_vec/vec_new_with_capacity_i64.ark",
    "stdlib_vec/vec_new_with_capacity_f64.ark",
    "stdlib_vec/vec_new_capacity_zero.ark",
    "stdlib_vec/vec_new_with_capacity_push.ark",
]

GUARD_RE = re.compile(
    r"global\.get 0"
    r".*?i32\.add"
    r".*?i32\.lt_u"
    r".*?select"
    r".*?global\.set 0",
    re.DOTALL,
)


def log(msg: str) -> None:
    print(f"[vec_new_overflow_guard] {msg}")


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def compile_fixture(fixture: str, out: Path) -> None:
    src = ROOT / "tests" / "fixtures" / fixture
    if not src.exists():
        raise FileNotFoundError(f"fixture missing: {src}")
    # The selfhost WASI sandbox only sees paths below REPO_ROOT, so use relative
    # paths both for the input source and the output wasm.
    src_rel = src.relative_to(ROOT)
    out_rel = out.relative_to(ROOT)
    cmd = [str(SELFHOST), "compile", str(src_rel), "--target", "wasm32", "--output", str(out_rel)]
    result = run(cmd)
    if result.returncode != 0:
        raise RuntimeError(
            f"compile failed for {fixture}:\n{result.stdout}\n{result.stderr}"
        )


def run_fixture(wasm: Path) -> None:
    wasmtime = shutil.which("wasmtime")
    if not wasmtime:
        raise RuntimeError("wasmtime not found in PATH")
    result = run([wasmtime, "run", str(wasm.relative_to(ROOT))])
    if result.returncode != 0:
        raise RuntimeError(
            f"wasmtime failed for {wasm}:\n{result.stdout}\n{result.stderr}"
        )


def check_guard(wasm: Path) -> None:
    wasm_tools = shutil.which("wasm-tools")
    if not wasm_tools:
        raise RuntimeError("wasm-tools not found in PATH")
    result = run([wasm_tools, "print", str(wasm.relative_to(ROOT))])
    if result.returncode != 0:
        raise RuntimeError(f"wasm-tools print failed for {wasm}:\n{result.stderr}")
    if not GUARD_RE.search(result.stdout):
        raise RuntimeError(
            f"overflow guard pattern not found in generated WAT for {wasm}"
        )


def main() -> int:
    if not SELFHOST.exists():
        log(f"selfhost wrapper not found: {SELFHOST}")
        return 1

    GATE_DIR.mkdir(parents=True, exist_ok=True)
    failures = []

    for fixture in FIXTURES:
        name = Path(fixture).stem
        wasm = GATE_DIR / f"{name}.wasm"
        try:
            log(f"compiling {fixture} ...")
            compile_fixture(fixture, wasm)
            log(f"running {wasm.name} ...")
            run_fixture(wasm)
            log(f"checking guard in {wasm.name} ...")
            check_guard(wasm)
            log(f"PASS: {fixture}")
        except Exception as exc:  # noqa: BLE001
            failures.append(f"{fixture}: {exc}")
            log(f"FAIL: {exc}")

    if failures:
        log("gate failed:")
        for failure in failures:
            log(f"  - {failure}")
        return 1

    log("all checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
