"""Regression tests for KEEP_CLOCK bootstrap-overlay type boundaries."""
from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from selfhost.checks import (  # noqa: E402
    _patch_bootstrap_driver_timing,
    _patch_bootstrap_mir_lower_phase_timing,
)


@pytest.fixture(autouse=True)
def _clear_keep_clock_groups(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("ARUKELLT_OVERLAY_KEEP_CLOCK_GROUPS", raising=False)


def test_keep_clock_default_uses_i32_ms_clock_and_wraps_getters() -> None:
    source = """fn compile_source(frontend: DriverFrontendResult) {
    let t0: i64 = pipeline_frontend::frontend_result_t0(frontend)
    let t_lex: i64 = pipeline_frontend::frontend_result_t_lex(frontend)
    let t_parse: i64 = pipeline_frontend::frontend_result_t_parse(frontend)
    let now = clock::monotonic_now()
}
"""
    result = _patch_bootstrap_driver_timing(source, keep_clock=True)
    assert (
        "let t0: i64 = i32_to_i64(pipeline_frontend::frontend_result_t0(frontend))"
        in result
    )
    assert "let now = i64_to_i32(clock::monotonic_now() / 1000000i64)" in result


def test_keep_clock_default_rewrites_record_fields_to_i32() -> None:
    source = """record DriverFrontendResult {
    t0: i64,
    t_lex: i64,
    t_parse: i64,
}
"""
    result = _patch_bootstrap_driver_timing(source, keep_clock=True)
    assert "    t0: i32," in result
    assert "    t0: i64," not in result


def test_keep_clock_narrow_groups_can_skip_field_rewrite() -> None:
    os.environ["ARUKELLT_OVERLAY_KEEP_CLOCK_GROUPS"] = "clock"
    try:
        source = """record DriverFrontendResult {
    t0: i64,
}
fn compile_source() {
    let now = clock::monotonic_now()
}
"""
        result = _patch_bootstrap_driver_timing(source, keep_clock=True)
        assert "    t0: i64," in result
        assert "i64_to_i32(clock::monotonic_now() / 1000000i64)" in result
    finally:
        os.environ.pop("ARUKELLT_OVERLAY_KEEP_CLOCK_GROUPS", None)


def test_final_emit_clock_widens_i32_ms_to_i64() -> None:
    source = """fn emit_timing_if_enabled() {
    debug::emit_phase_timing(
        t0,
        t_lex,
        t_parse,
        t_resolve,
        t_typecheck,
        t_lower,
        t_mir_opt,
        t_mir_verify,
        clock::monotonic_now()
    )
}
"""
    result = _patch_bootstrap_driver_timing(source, keep_clock=True)
    assert "i32_to_i64(i64_to_i32(clock::monotonic_now() / 1000000i64))" in result


def test_keep_clock_phase_timing_ms_skips_second_divide() -> None:
    source = """fn phase_timing_ms(start: i64, end: i64) -> i32 {
    let diff = end - start
    i64_to_i32(diff / 1000000i64)
}
"""
    result = _patch_bootstrap_driver_timing(source, keep_clock=True)
    assert "i64_to_i32(diff)" in result
    assert "i64_to_i32(diff / 1000000i64)" not in result


def test_keep_clock_narrows_i64_zero_timestamps_for_i32_slots() -> None:
    source = """fn run_lex_parse() {
    return frontend_stop(err, t0, 0i64, 0i64)
}

fn backend_resolve() {
    return DriverResolveResult_new(
        true,
        err,
        load_state,
        resolve_ctx,
        0i64
    )
}
"""
    result = _patch_bootstrap_driver_timing(source, keep_clock=True)
    assert "t0, 0, 0)" in result
    assert "0i64" not in result


def test_stub_narrows_i64_zero_timestamps_for_i32_slots() -> None:
    source = """fn run_lex_parse() {
    return frontend_stop(err, t0, 0i64, 0i64)
}
"""
    result = _patch_bootstrap_driver_timing(source, keep_clock=False)
    assert "t0, 0, 0)" in result
    assert "0i64" not in result


def test_keep_clock_mir_lower_phase_keeps_ns_and_ms_divide() -> None:
    source = """fn mir_lower_phase_now() -> i64 {
    clock::monotonic_now()
}

fn mir_lower_phase_ms(t_start: i64, t_end: i64) -> i32 {
    let diff = t_end - t_start
    i64_to_i32(diff / 1000000i64)
}
"""
    result = _patch_bootstrap_mir_lower_phase_timing(source, keep_clock=True)
    assert "clock::monotonic_now()" in result
    assert "clock::monotonic_now() / 1000000i64" not in result
    assert "i64_to_i32(diff / 1000000i64)" in result
