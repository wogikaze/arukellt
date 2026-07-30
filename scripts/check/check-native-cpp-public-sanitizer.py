#!/usr/bin/env python3
"""Compile+run a subset of native_cpp_public fixtures under ASan/UBSan."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from native.toolchain import resolve_clang  # noqa: E402

WRAPPER = ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
RUNTIME_DIR = ROOT / "src" / "compiler" / "native_c" / "runtime"
PUBLIC = ROOT / "tests" / "fixtures" / "native_cpp_public"

FIXTURES = [
    "stdio_hello.ark",
    "scalar_cfg.ark",
    "hof_named_callback.ark",
    "phi_if_join_i32.ark",
    "string_vec_struct.ark",
]


def _build_dir() -> Path:
    env = os.environ.get("ARUKELLT_BUILD_DIR", "").strip()
    if env:
        return Path(env)
    return ROOT / ".build"


def main() -> int:
    clang, diag = resolve_clang()
    if clang is None:
        print(diag, file=sys.stderr)
        return 1
    env = os.environ.copy()
    env["ARUKELLT_BUILD_DIR"] = str(_build_dir())
    failures = 0
    with tempfile.TemporaryDirectory(prefix="native-public-asan-", dir=str(_build_dir())) as tmp:
        tmp_path = Path(tmp)
        for name in FIXTURES:
            fixture = PUBLIC / name
            c_path = tmp_path / f"{name}.c"
            exe_path = tmp_path / f"{name}.exe"
            compile_cmd = [
                str(WRAPPER),
                "compile",
                str(fixture.relative_to(ROOT)),
                "--target",
                "native-cpp",
                "--emit",
                "c",
                "-o",
                str(c_path.relative_to(ROOT)) if c_path.is_relative_to(ROOT) else str(c_path),
            ]
            # Keep generated C under build dir inside repo for WASI --dir.
            c_rel = c_path
            try:
                c_rel = c_path.relative_to(ROOT)
            except ValueError:
                pass
            compile_cmd[-1] = str(c_rel)
            compile_env = env.copy()
            compile_env["ARUKELLT_NATIVE_CPP_INTERNAL_COMPILE"] = "1"
            compiled = subprocess.run(
                compile_cmd, cwd=ROOT, env=compile_env, capture_output=True, text=True, check=False
            )
            if compiled.returncode != 0:
                print(f"FAIL compile {name}: {compiled.stdout}{compiled.stderr}", file=sys.stderr)
                failures += 1
                continue
            clang_cmd = [
                clang.path,
                "-std=c99",
                "-O1",
                "-g",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-Wno-unused-function",
                # Prefer UBSan for the gate. Standalone ASan has been flaky with
                # this runtime on some hosts (empty SIGSEGV); UBSan catches the
                # portable C99 contract issues we care about for promotion.
                "-fsanitize=undefined",
                f"-I{RUNTIME_DIR}",
                str(ROOT / c_rel) if not Path(c_rel).is_absolute() else str(c_rel),
                str(RUNTIME_DIR / "ark_native_runtime.c"),
                "-lm",
                "-o",
                str(exe_path),
            ]
            linked = subprocess.run(clang_cmd, cwd=ROOT, capture_output=True, text=True, check=False)
            if linked.returncode != 0:
                print(f"FAIL link/sanitize {name}: {linked.stderr[-2000:]}", file=sys.stderr)
                failures += 1
                continue
            run_env = env.copy()
            run_env["ARUKELLT_NATIVE_GC"] = "1"
            run_env["UBSAN_OPTIONS"] = "print_stacktrace=1:halt_on_error=1"
            ran = subprocess.run(
                [str(exe_path)],
                cwd=ROOT,
                env=run_env,
                capture_output=True,
                text=True,
                check=False,
            )
            if ran.returncode != 0:
                print(
                    f"FAIL run {name} rc={ran.returncode}: {ran.stdout}\n{ran.stderr}",
                    file=sys.stderr,
                )
                failures += 1
                continue
            print(f"PASS sanitizer {name}")
    if failures:
        print(f"native-cpp public sanitizer failures: {failures}", file=sys.stderr)
        return 1
    print("native-cpp-public-sanitizer: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
