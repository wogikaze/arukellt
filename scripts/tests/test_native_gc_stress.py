#!/usr/bin/env python3
"""GC stress fixtures for native-cpp root liveness / mark-sweep (#832 follow-on)."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "fixtures" / "native_gc_stress"
BUILD = Path(os.environ.get("ARUKELLT_BUILD_DIR", str(ROOT / ".build-native-recovery")))
NATIVE = BUILD / "selfhost" / "native" / "arukellt-native"
CC = os.environ.get("ARUKELLT_CC", "clang-16")
RUNTIME_C = ROOT / "src" / "compiler" / "native_c" / "runtime" / "ark_native_runtime.c"
RUNTIME_I = ROOT / "src" / "compiler" / "native_c" / "runtime"


def _run(cmd: list[str], *, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
        env=env,
    )


def test_native_gc_stress_fixtures_compile_and_run() -> None:
    assert NATIVE.is_file(), f"missing native executor binary: {NATIVE}"
    assert FIXTURES.is_dir()
    fixtures = sorted(FIXTURES.glob("*.ark"))
    assert fixtures, "no native_gc_stress fixtures"

    with tempfile.TemporaryDirectory(prefix="native-gc-stress-") as tmp:
        tmp_path = Path(tmp)
        for fixture in fixtures:
            wasm_out = tmp_path / f"{fixture.stem}.wasm"
            env = os.environ.copy()
            env["ARUKELLT_NATIVE_GC"] = "1"
            env["ARUKELLT_NATIVE_GC_THRESHOLD_BYTES"] = "65536"
            compile_wasm = _run(
                [
                    str(NATIVE),
                    "compile",
                    str(fixture),
                    "--target",
                    "wasm32",
                    "--wasi-version",
                    "wasi-p1",
                    "--output",
                    str(wasm_out),
                ],
                env=env,
            )
            assert compile_wasm.returncode == 0, (
                f"{fixture.name} wasm compile failed:\n"
                f"{compile_wasm.stderr[-2000:]}\n{compile_wasm.stdout[-2000:]}"
            )
            assert wasm_out.is_file()

            c_out = tmp_path / f"{fixture.stem}.c"
            compile_c = _run(
                [
                    str(NATIVE),
                    "compile",
                    str(fixture),
                    "--target",
                    "native-cpp",
                    "--output",
                    str(c_out),
                ],
                env=env,
            )
            assert compile_c.returncode == 0, (
                f"{fixture.name} native-cpp compile failed:\n"
                f"{compile_c.stderr[-2000:]}\n{compile_c.stdout[-2000:]}"
            )
            exe = tmp_path / fixture.stem
            link = _run(
                [
                    CC,
                    "-std=c99",
                    "-O1",
                    f"-I{RUNTIME_I}",
                    str(c_out),
                    str(RUNTIME_C),
                    "-o",
                    str(exe),
                ]
            )
            assert link.returncode == 0, f"{fixture.name} link failed:\n{link.stderr[-2000:]}"

            stats = tmp_path / f"{fixture.stem}.gc-stats.json"
            run_env = os.environ.copy()
            run_env["ARUKELLT_NATIVE_GC"] = "1"
            run_env["ARUKELLT_NATIVE_GC_THRESHOLD_BYTES"] = "65536"
            run_env["ARUKELLT_NATIVE_GC_STATS_PATH"] = str(stats)
            executed = _run([str(exe)], env=run_env)
            assert executed.returncode == 0, (
                f"{fixture.name} native run failed:\n"
                f"{executed.stderr[-2000:]}\n{executed.stdout[-2000:]}"
            )
            if fixture.stem == "dead_reclaim":
                assert stats.is_file(), "dead_reclaim missing GC stats"
                payload = json.loads(stats.read_text(encoding="utf-8"))
                assert int(payload.get("gc_collection_count", 0)) >= 1
                assert int(payload.get("gc_reclaimed_object_bytes", 0)) > 0


if __name__ == "__main__":
    try:
        test_native_gc_stress_fixtures_compile_and_run()
    except AssertionError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
    print("native_gc_stress: PASS")
