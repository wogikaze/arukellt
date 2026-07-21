"""Smoke: KEEP_CLOCK s2 validates and ``compile --time`` reports non-zero totals.

Requires a prebuilt ``.build/selfhost/arukellt-s2-clock.wasm`` (from
``build_clock_capable_s2``). Skips when the artifact is absent so unit CI
without a multi-minute KEEP_CLOCK rebuild still passes.
"""
from __future__ import annotations

import re
import sys
import time
from pathlib import Path

import pytest

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
_ROOT = _SCRIPTS_DIR.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from selfhost.checks import (  # noqa: E402
    CLOCK_S2_WASM_REL,
    S2_RUNTIME_WASM_REL,
    _find_wasmtime,
    _run,
    _wasm_run_cmd,
    _wasm_tools_validate,
)

_PHASE_RE = re.compile(r"\[arukellt\]\s+([A-Za-z0-9_.]+):\s*([0-9]+(?:\.[0-9]+)?)ms")
_HELLO = "tests/fixtures/hello/hello.ark"
_CLOCK = _ROOT / CLOCK_S2_WASM_REL
_RUNTIME = _ROOT / S2_RUNTIME_WASM_REL
_PRE64 = _ROOT / ".build/selfhost/arukellt-s2-clock-wasm32.wasm"


def _timings(text: str) -> dict[str, float]:
    return {m.group(1): float(m.group(2)) for m in _PHASE_RE.finditer(text)}


def _compile_with_time(compiler: Path, out_rel: str) -> tuple[int, dict[str, float], float, bool]:
    out = _ROOT / out_rel
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.is_file():
        out.unlink()
    wasmtime = _find_wasmtime()
    assert wasmtime, "wasmtime required for KEEP_CLOCK --time smoke"
    cmd = _wasm_run_cmd(
        wasmtime,
        compiler,
        _ROOT,
        [
            "compile",
            _HELLO,
            "--target",
            "wasm32-gc",
            "--wasi-version",
            "wasi-p1",
            "--time",
            "-o",
            out_rel,
        ],
    )
    started = time.perf_counter()
    proc = _run(cmd, _ROOT, timeout=180)
    wall_ms = (time.perf_counter() - started) * 1000.0
    text = (proc.stderr or "") + "\n" + (proc.stdout or "")
    return proc.returncode, _timings(text), wall_ms, out.is_file()


@pytest.mark.skipif(not _CLOCK.is_file(), reason="arukellt-s2-clock.wasm not built")
def test_keep_clock_artifacts_validate() -> None:
    rc, msg = _wasm_tools_validate(_CLOCK)
    assert rc in (0, 2), msg
    if _PRE64.is_file():
        rc_pre, msg_pre = _wasm_tools_validate(_PRE64)
        assert rc_pre in (0, 2), msg_pre


@pytest.mark.skipif(not _CLOCK.is_file(), reason="arukellt-s2-clock.wasm not built")
def test_keep_clock_hello_time_is_nonzero_and_near_wall() -> None:
    code, timings, wall_ms, out_ok = _compile_with_time(
        _CLOCK, ".ark-debug/hello-keep-clock-time-test.wasm"
    )
    assert code == 0
    assert out_ok
    total = timings.get("total")
    assert total is not None and total > 0, timings
    assert wall_ms > 0
    ratio = total / wall_ms
    assert 0.05 <= ratio <= 1.5, f"total={total} wall_ms={wall_ms} ratio={ratio}"


@pytest.mark.skipif(
    not (_CLOCK.is_file() and _RUNTIME.is_file()),
    reason="clock or stub runtime wasm missing",
)
def test_stub_runtime_time_stays_zero_while_compile_succeeds() -> None:
    code, timings, _wall_ms, out_ok = _compile_with_time(
        _RUNTIME, ".ark-debug/hello-stub-time-test.wasm"
    )
    assert code == 0
    assert out_ok
    assert timings.get("total", 0.0) == 0.0, timings
