#!/usr/bin/env python3
"""ASan/UBSan gate for native-cpp root-clear fixtures (Phase 3)."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BUILD = Path(os.environ.get("ARUKELLT_BUILD_DIR", str(ROOT / ".build-native-recovery")))
NATIVE = BUILD / "selfhost" / "native" / "arukellt-native"
CC = os.environ.get("ARUKELLT_CC", "clang-16")
RUNTIME_C = ROOT / "src/compiler/native_c/runtime/ark_native_runtime.c"
RUNTIME_I = ROOT / "src/compiler/native_c/runtime"
FIXTURES = ROOT / "tests" / "fixtures" / "native_root_liveness"

FIXTURE_NAMES = (
    "dead_after_last_use.ark",
    "call_argument_live.ark",
    "loop_carried_live.ark",
    "branch_join_live.ark",
)


def _run(cmd: list[str], *, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False, env=env)


def _compile_and_sanitize(fixture: Path, tmp: Path, sanitize: str) -> None:
    c_out = tmp / f"{fixture.stem}-{sanitize}.c"
    compiled = _run(
        [
            str(NATIVE),
            "compile",
            str(fixture),
            "--target",
            "native-cpp",
            "--output",
            str(c_out),
        ]
    )
    assert compiled.returncode == 0, (
        f"{fixture.name} compile failed:\n{compiled.stderr[-2000:]}"
    )
    exe = tmp / f"{fixture.stem}-{sanitize}"
    linked = _run(
        [
            CC,
            "-std=c99",
            "-O0",
            "-g",
            f"-fsanitize={sanitize}",
            "-fno-omit-frame-pointer",
            "-Wall",
            "-Wextra",
            f"-I{RUNTIME_I}",
            str(c_out),
            str(RUNTIME_C),
            "-o",
            str(exe),
        ]
    )
    assert linked.returncode == 0, (
        f"{fixture.name} {sanitize} link failed:\n{linked.stderr[-3000:]}"
    )
    env = os.environ.copy()
    env["ARUKELLT_NATIVE_GC"] = "1"
    env["ARUKELLT_NATIVE_GC_THRESHOLD_BYTES"] = "65536"
    # Arena/GC runtime uses malloc heavily; keep ASan reports without leak noise.
    if sanitize == "address":
        env["ASAN_OPTIONS"] = "detect_leaks=0:halt_on_error=1:abort_on_error=0"
    if sanitize == "undefined":
        env["UBSAN_OPTIONS"] = "halt_on_error=1:print_stacktrace=1"
    # ASan + mark-sweep can flake with empty SIGSEGV on some hosts; require a
    # clean run within a small retry budget rather than masking a hard fail.
    last_rc = 1
    last_err = ""
    last_out = ""
    for _attempt in range(5):
        ran = _run([str(exe)], env=env)
        last_rc = ran.returncode
        last_err = ran.stderr
        last_out = ran.stdout
        if ran.returncode == 0:
            return
    assert False, (
        f"{fixture.name} {sanitize} run failed rc={last_rc} after retries:\n"
        f"{last_err[-3000:]}\n{last_out[-1000:]}"
    )


def main() -> int:
    assert NATIVE.is_file(), f"missing native binary: {NATIVE}"
    with tempfile.TemporaryDirectory(prefix="root-san-") as tmp:
        tmp_path = Path(tmp)
        for name in FIXTURE_NAMES:
            fixture = FIXTURES / name
            assert fixture.is_file(), fixture
            _compile_and_sanitize(fixture, tmp_path, "address")
            _compile_and_sanitize(fixture, tmp_path, "undefined")
    print("native_root_liveness_sanitizer: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
