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


def test_stub_overlay_widens_backend_phase_timestamps() -> None:
    source = """fn run_backend(source: String, config: DriverConfig, decls: Vec<AstNode>, t0: i64, t_lex: i64, t_parse: i64) {
    let t_resolve = resolve_result_t_resolve(resolved)
    debug::note_phase_if_enabled(config, "driver.resolve", t_parse, t_resolve)
    let t_typecheck = typecheck_result_t_typecheck(checked)
    debug::note_phase_if_enabled(config, "driver.typecheck", t_resolve, t_typecheck)
    let t_lower = clock::monotonic_now()
    debug::note_phase_if_enabled(config, "driver.lower", t_typecheck, t_lower)
    let t_mir_opt = clock::monotonic_now()
    debug::note_phase_if_enabled(config, "driver.mir_opt", t_lower, t_mir_opt)
    let t_mir_verify = clock::monotonic_now()
    debug::note_phase_if_enabled(config, "driver.mir_verify", t_mir_opt, t_mir_verify)
}
"""
    from selfhost.checks import _patch_bootstrap_driver_timing_backend

    result = _patch_bootstrap_driver_timing_backend(source)
    assert '"driver.resolve", t_parse, i32_to_i64(t_resolve)' in result
    assert '"driver.typecheck", i32_to_i64(t_resolve), i32_to_i64(t_typecheck)' in result
    assert '"driver.lower", i32_to_i64(t_typecheck), i32_to_i64(t_lower)' in result
    assert '"driver.mir_opt", i32_to_i64(t_lower), i32_to_i64(t_mir_opt)' in result
    assert '"driver.mir_verify", i32_to_i64(t_mir_opt), i32_to_i64(t_mir_verify)' in result


def test_stub_overlay_widens_current_emit_output_timestamps() -> None:
    source = """fn run_backend(source: String, config: DriverConfig, decls: Vec<AstNode>, t0: i64, t_lex: i64, t_parse: i64) {
    emit::emit_output(
        mir_module,
        config,
        t0,
        t_lex,
        t_parse,
        backend_resolve::resolve_result_t_resolve(resolved),
        t_typecheck,
        t_lower,
        t_mir_opt,
        t_mir_verify,
        wit_decls
    )
}
"""
    from selfhost.checks import _patch_bootstrap_driver_timing_backend

    result = _patch_bootstrap_driver_timing_backend(source)
    assert "i32_to_i64(backend_resolve::resolve_result_t_resolve(resolved))" in result
    assert "i32_to_i64(t_typecheck)" in result
    assert "i32_to_i64(t_mir_verify)" in result


def test_stub_overlay_widens_frontend_phase_timestamps() -> None:
    source = """fn run_lex_parse(config: DriverConfig, t0: i32) {
    let t_lex = 0
    debug::note_phase_if_enabled(config, "driver.lex", t0, t_lex)
    let t_parse = 0
    debug::note_phase_if_enabled(config, "driver.parse", t_lex, t_parse)
}
"""
    from selfhost.checks import _patch_bootstrap_driver_timing_clock

    result = _patch_bootstrap_driver_timing_clock(source, keep_clock=False)
    assert '"driver.lex", i32_to_i64(t0), i32_to_i64(t_lex)' in result
    assert '"driver.parse", i32_to_i64(t_lex), i32_to_i64(t_parse)' in result


def test_stub_overlay_widens_emit_phase_timestamps() -> None:
    source = """fn emit_wasm_output(config: DriverConfig, t_mir_verify: i64) {
    let t_emit_enter = clock::monotonic_now()
    debug::note_phase_if_enabled(config, "driver.emit.enter", t_mir_verify, t_emit_enter)
    let t_bind = clock::monotonic_now()
    debug::note_phase_if_enabled(config, "driver.emit.wit_bind", t_emit_enter, t_bind)
}
"""
    from selfhost.checks import _patch_bootstrap_driver_timing_emit

    result = _patch_bootstrap_driver_timing_emit(source, keep_clock=False)
    assert '"driver.emit.enter", t_mir_verify, i32_to_i64(t_emit_enter)' in result
    assert '"driver.emit.wit_bind", i32_to_i64(t_emit_enter), i32_to_i64(t_bind)' in result
