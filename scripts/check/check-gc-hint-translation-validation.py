#!/usr/bin/env python3
"""Fail-closed source contract for every enabled MIR optimizer transformation."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MIR_OPT = ROOT / "src" / "compiler" / "mir_opt"
GENERIC = MIR_OPT / "translation_validation.ark"
ASYNC = MIR_OPT / "async_lower" / "mod.ark"
ASYNC_VALIDATOR = MIR_OPT / "async_lower" / "translation_validation.ark"
GC_HINT = MIR_OPT / "gc_hint_core.ark"
LICM = MIR_OPT / "licm_core.ark"
LICM_VALIDATOR = MIR_OPT / "licm_translation_validation.ark"
UNROLL = MIR_OPT / "loop_unroll.ark"
UNROLL_VALIDATOR = MIR_OPT / "loop_unroll_translation_validation.ark"
ORCHESTRATE = MIR_OPT / "orchestrate.ark"
SUMMARY = MIR_OPT / "summary_record.ark"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def main() -> int:
    generic = GENERIC.read_text(encoding="utf-8")
    async_lower = ASYNC.read_text(encoding="utf-8")
    async_validator = ASYNC_VALIDATOR.read_text(encoding="utf-8")
    gc_hint = GC_HINT.read_text(encoding="utf-8")
    licm = LICM.read_text(encoding="utf-8")
    licm_validator = LICM_VALIDATOR.read_text(encoding="utf-8")
    unroll = UNROLL.read_text(encoding="utf-8")
    unroll_validator = UNROLL_VALIDATOR.read_text(encoding="utf-8")
    orchestrate = ORCHESTRATE.read_text(encoding="utf-8")
    summary = SUMMARY.read_text(encoding="utf-8")

    required_instruction_fields = (
        "MirInst_op",
        "MirInst_dest",
        "MirInst_arg0",
        "MirInst_arg1",
        "MirInst_int_val",
        "MirInst_float_val",
        "MirInst_str_val",
        "MirInst_val_type",
        "mir_inst_func_id_raw",
        "mir_inst_result_type_count",
        "mir_inst_result_type_at",
    )
    require(generic, "fn mir_translation_inst_equal", "shared instruction equality")
    for field in required_instruction_fields:
        require(generic, field, f"shared instruction field comparison {field}")
    require(generic, "fn mir_translation_vector_equal", "shared vector equality")
    require(generic, "fn mir_translation_insert_only_valid", "insert-only policy")

    require(
        async_validator,
        "fn async_lower_translation_valid",
        "async-lowering independent validator",
    )
    require(
        async_validator,
        "MIR_AWAIT() || op == opcodes::MIR_FUTURE_NEW()",
        "async operation classification",
    )
    require(
        async_lower,
        "if translation_validation::async_lower_translation_valid(before, filtered)",
        "async-lowering guarded application",
    )
    require(
        async_lower,
        "MirBlock_set_instructions(block, before)",
        "async-lowering original restoration",
    )

    require(gc_hint, "fn gc_hint_translation_valid", "gc_hint validator")
    require(
        gc_hint,
        "block_inst_mutation::MirBlock_set_instructions(block, insts)",
        "gc_hint original restoration",
    )
    require(
        gc_hint,
        "OptimizationSummary_add_translation_validation_failure",
        "gc_hint failure propagation",
    )

    require(
        licm_validator,
        "fn licm_translation_valid",
        "LICM independent validator",
    )
    require(
        licm_validator,
        "licm_collect_loop_modified",
        "LICM dependency validation",
    )
    require(
        licm,
        "if licm_translation_validation::licm_translation_valid",
        "LICM guarded candidate application",
    )
    require(
        licm,
        "validation_failures = validation_failures + 1",
        "LICM failure fallback",
    )
    require(
        licm,
        "OptimizationSummary_add_translation_validation_failure",
        "LICM failure propagation",
    )

    require(
        unroll_validator,
        "fn loop_unroll_translation_valid",
        "loop-unroll independent validator",
    )
    require(
        unroll_validator,
        "MirInst_const_i32",
        "loop counter substitution validation",
    )
    require(
        unroll,
        "if loop_unroll_translation_validation::loop_unroll_translation_valid",
        "loop-unroll guarded candidate application",
    )
    require(
        unroll,
        "LoopUnrollResult_new(insts, 0, 1)",
        "loop-unroll original restoration",
    )
    require(
        unroll,
        "OptimizationSummary_add_translation_validation_failure",
        "loop-unroll failure propagation",
    )
    if "mir_opt_const_fold_insts(out)" in unroll:
        raise ValueError(
            "loop unroll must not hide an unvalidated const-fold transformation"
        )

    active_orchestrate = [
        line.strip()
        for line in orchestrate.splitlines()
        if line.strip() and not line.strip().startswith("//")
    ]
    if "stdlib_inline::stdlib_inline_module(m)" in active_orchestrate:
        raise ValueError("stdlib inline is enabled without an independent validator")
    require(
        orchestrate,
        "async_lower::async_lower_optimize_module(m)",
        "validated async-lowering invocation",
    )
    require(orchestrate, "loop_unroll::run_loop_unroll(f)", "validated unroll invocation")
    require(orchestrate, "licm::run_licm(f)", "validated LICM invocation")
    require(orchestrate, "gc_hint::run_gc_hint(f)", "validated gc_hint invocation")

    require(summary, "translation_validation_failures: i32", "summary field")
    require(
        summary,
        "fn OptimizationSummary_translation_validation_failures",
        "summary accessor",
    )
    require(
        summary,
        "fn OptimizationSummary_add_translation_validation_failure",
        "summary accumulator",
    )
    require(
        summary,
        "a.translation_validation_failures + b.translation_validation_failures",
        "summary merge",
    )

    print(
        "mir-opt-translation-validation: PASS: "
        "async_lower gc_hint licm loop_unroll; stdlib_inline=disabled"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"mir-opt-translation-validation: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
