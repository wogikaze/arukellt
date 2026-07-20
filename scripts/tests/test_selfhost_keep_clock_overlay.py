"""Regression tests for KEEP_CLOCK bootstrap-overlay type boundaries."""
from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

_SCRIPTS_DIR = Path(__file__).resolve().parent.parent
if str(_SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS_DIR))

from selfhost.checks import _patch_bootstrap_driver_timing  # noqa: E402


@pytest.fixture(autouse=True)
def _clear_keep_clock_groups(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("ARUKELLT_OVERLAY_KEEP_CLOCK_GROUPS", raising=False)


def test_keep_clock_default_keeps_i64_getters_and_wraps_clock_to_i64_ms() -> None:
    source = """fn compile_source(frontend: DriverFrontendResult) {
    let t0: i64 = pipeline_frontend::frontend_result_t0(frontend)
    let t_lex: i64 = pipeline_frontend::frontend_result_t_lex(frontend)
    let t_parse: i64 = pipeline_frontend::frontend_result_t_parse(frontend)
    let now = clock::monotonic_now()
}
"""
    result = _patch_bootstrap_driver_timing(source, keep_clock=True)
    assert "let t0: i64 = pipeline_frontend::frontend_result_t0(frontend)" in result
    assert "i32_to_i64(pipeline_frontend::frontend_result_t0" not in result
    assert "i32_to_i64(i64_to_i32(clock::monotonic_now() / 1000000i64))" in result


def test_keep_clock_default_does_not_rewrite_record_fields() -> None:
    source = """record DriverFrontendResult {
    t0: i64,
    t_lex: i64,
    t_parse: i64,
}
"""
    result = _patch_bootstrap_driver_timing(source, keep_clock=True)
    assert "    t0: i64," in result
    assert "    t0: i32," not in result


def test_keep_clock_legacy_groups_still_wrap_backend_getters() -> None:
    os.environ["ARUKELLT_OVERLAY_KEEP_CLOCK_GROUPS"] = "clock,fields,sigs,backend,emit"
    try:
        source = """fn compile_source(frontend: DriverFrontendResult) {
    let t0: i64 = pipeline_frontend::frontend_result_t0(frontend)
}
"""
        result = _patch_bootstrap_driver_timing(source, keep_clock=True)
        assert (
            "let t0: i64 = i32_to_i64(pipeline_frontend::frontend_result_t0(frontend))"
            in result
        )
    finally:
        os.environ.pop("ARUKELLT_OVERLAY_KEEP_CLOCK_GROUPS", None)


def test_final_emit_clock_is_i64_ms_under_default_keep_clock() -> None:
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
