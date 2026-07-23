#!/usr/bin/env python3
"""Planner / clear-marker checks for native-cpp root liveness (Phase 2–3)."""

from __future__ import annotations

import os
import re
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

FN_MARKER = re.compile(
    r"/\* ARK_ROOT_LIVENESS fn analyzed=(\d+) skipped=(\d+) "
    r"frames=(\d+) safepoints=(\d+) sites_planned=(\d+) "
    r"assigns_planned=(\d+) sites_emitted=(\d+) assigns_emitted=(\d+) "
    r"peak_slots=(\d+) planner_bytes=(\d+) entry_nulls=(\d+) emit=(\d+) \*/"
)
CLEAR_MARKER = re.compile(r"/\* ark-root-clear inst=(\d+) local=(\d+) \*/")


def _run(cmd: list[str], *, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False, env=env)


def _compile_fixture(fixture: Path, tmp: Path) -> str:
    assert NATIVE.is_file(), f"missing native binary: {NATIVE}"
    c_out = tmp / f"{fixture.stem}.c"
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
        f"{fixture.name} compile failed:\n{compiled.stderr[-2000:]}\n{compiled.stdout[-2000:]}"
    )
    return c_out.read_text(encoding="utf-8", errors="replace")


def _main_marker(text: str) -> re.Match[str]:
    matches = list(FN_MARKER.finditer(text))
    assert matches, "missing ARK_ROOT_LIVENESS fn marker"
    # Prefer the marker with emit=1 / largest assigns when multiple functions exist.
    return max(matches, key=lambda m: (int(m.group(12)), int(m.group(6))))


def test_dead_after_last_use_plans_clear() -> None:
    fixture = FIXTURES / "dead_after_last_use.ark"
    with tempfile.TemporaryDirectory(prefix="root-live-") as tmp:
        text = _compile_fixture(fixture, Path(tmp))
    marker = _main_marker(text)
    assert int(marker.group(2)) == 0, "skipped must be 0"
    assert int(marker.group(6)) >= 1, "expected planned assignments"
    assert int(marker.group(12)) == 0, "emit stays off until structured CFG liveness"
    assert int(marker.group(8)) == 0, "no emitted assignments in shadow mode"
    assert not CLEAR_MARKER.search(text), "no ark-root-clear markers while emit disabled"


def test_call_argument_kept_live() -> None:
    fixture = FIXTURES / "call_argument_live.ark"
    with tempfile.TemporaryDirectory(prefix="root-live-") as tmp:
        text = _compile_fixture(fixture, Path(tmp))
        c_out = Path(tmp) / f"{fixture.stem}.c"
        c_out.write_text(text, encoding="utf-8")
        exe = Path(tmp) / fixture.stem
        linked = _run(
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
        assert linked.returncode == 0, linked.stderr[-2000:]
        env = os.environ.copy()
        env["ARUKELLT_NATIVE_GC"] = "1"
        env["ARUKELLT_NATIVE_GC_THRESHOLD_BYTES"] = "65536"
        ran = _run([str(exe)], env=env)
        assert ran.returncode == 0, ran.stderr[-2000:] + ran.stdout[-2000:]


def test_control_flow_fixtures_compile_without_emit() -> None:
    """Structured control must not enable clears until CFG successors exist."""
    names = (
        "branch_join_live.ark",
        "loop_carried_live.ark",
        "early_return_live.ark",
    )
    with tempfile.TemporaryDirectory(prefix="root-live-") as tmp:
        tmp_path = Path(tmp)
        for name in names:
            fixture = FIXTURES / name
            text = _compile_fixture(fixture, tmp_path)
            marker = _main_marker(text)
            assert int(marker.group(2)) == 0, f"{name}: skipped"
            assert int(marker.group(12)) == 0, f"{name}: emit must stay off for control-flow"
            c_out = tmp_path / f"{fixture.stem}.c"
            c_out.write_text(text, encoding="utf-8")
            exe = tmp_path / fixture.stem
            linked = _run(
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
            assert linked.returncode == 0, f"{name} link:\n{linked.stderr[-2000:]}"
            env = os.environ.copy()
            env["ARUKELLT_NATIVE_GC"] = "1"
            env["ARUKELLT_NATIVE_GC_THRESHOLD_BYTES"] = "65536"
            ran = _run([str(exe)], env=env)
            assert ran.returncode == 0, f"{name} run:\n{ran.stderr[-2000:]}"


def test_straight_line_overwrite_runs_in_shadow_mode() -> None:
    fixture = FIXTURES / "overwrite_old_dead.ark"
    with tempfile.TemporaryDirectory(prefix="root-live-") as tmp:
        tmp_path = Path(tmp)
        text = _compile_fixture(fixture, tmp_path)
        marker = _main_marker(text)
        assert int(marker.group(12)) == 0
        assert int(marker.group(6)) >= 0
        c_out = tmp_path / f"{fixture.stem}.c"
        c_out.write_text(text, encoding="utf-8")
        exe = tmp_path / fixture.stem
        linked = _run(
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
        assert linked.returncode == 0, linked.stderr[-2000:]
        env = os.environ.copy()
        env["ARUKELLT_NATIVE_GC"] = "1"
        env["ARUKELLT_NATIVE_GC_THRESHOLD_BYTES"] = "65536"
        ran = _run([str(exe)], env=env)
        assert ran.returncode == 0, ran.stderr[-2000:]


if __name__ == "__main__":
    try:
        test_dead_after_last_use_plans_clear()
        test_call_argument_kept_live()
        test_control_flow_fixtures_compile_without_emit()
        test_straight_line_overwrite_runs_in_shadow_mode()
    except AssertionError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
    print("native_root_liveness_planner: PASS")
