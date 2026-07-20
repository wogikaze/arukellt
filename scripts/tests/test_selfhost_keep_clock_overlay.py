"""Regression tests for KEEP_CLOCK bootstrap-overlay type boundaries."""
from __future__ import annotations

import sys
from pathlib import Path

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from selfhost.checks import _patch_bootstrap_driver_timing  # noqa: E402


def test_frontend_timestamp_getters_are_extended_for_backend_i64_locals() -> None:
    source = """fn compile_source(frontend: DriverFrontendResult) {
    let t0: i64 = pipeline_frontend::frontend_result_t0(frontend)
    let t_lex: i64 = pipeline_frontend::frontend_result_t_lex(frontend)
    let t_parse: i64 = pipeline_frontend::frontend_result_t_parse(frontend)
}
"""
    result = _patch_bootstrap_driver_timing(source, keep_clock=True)
    assert (
        "let t0: i64 = i32_to_i64(pipeline_frontend::frontend_result_t0(frontend))"
        in result
    )
    assert (
        "let t_lex: i64 = i32_to_i64(pipeline_frontend::frontend_result_t_lex(frontend))"
        in result
    )
    assert (
        "let t_parse: i64 = i32_to_i64(pipeline_frontend::frontend_result_t_parse(frontend))"
        in result
    )


def test_final_emit_clock_is_extended_for_i64_timing_parameter() -> None:
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
