#!/usr/bin/env python3
"""Fail-closed source contract for gc_hint translation validation."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GC_HINT = ROOT / "src" / "compiler" / "mir_opt" / "gc_hint_core.ark"
SUMMARY = ROOT / "src" / "compiler" / "mir_opt" / "summary_record.ark"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def main() -> int:
    gc_hint = GC_HINT.read_text(encoding="utf-8")
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
    require(gc_hint, "fn gc_hint_instruction_equal", "instruction equality validator")
    for field in required_instruction_fields:
        require(gc_hint, field, f"instruction field comparison {field}")

    require(gc_hint, "fn gc_hint_translation_valid", "translation validator")
    require(
        gc_hint,
        "fn gc_hint_is_canonical_inserted_hint",
        "canonical hint predicate",
    )
    require(
        gc_hint,
        "gc_hint_instruction_equal(inst, inst_gc_hint::MirInst_gc_hint_short_lived())",
        "canonical short-lived hint comparison",
    )
    require(
        gc_hint,
        "if gc_hint_is_canonical_inserted_hint(candidate)",
        "canonical insertion allowance",
    )
    require(
        gc_hint,
        "block_inst_mutation::MirBlock_set_instructions(block, insts)",
        "original-block restoration",
    )
    require(
        gc_hint,
        "translation_validation_failures = translation_validation_failures + 1",
        "failure accounting",
    )
    require(
        gc_hint,
        "OptimizationSummary_add_translation_validation_failure",
        "summary failure propagation",
    )

    require(
        gc_hint,
        "fn gc_hint_resolve_use_target",
        "constructor-temp alias resolution",
    )
    require(
        gc_hint,
        "op == opcodes::MIR_LOCAL_SET() && arg1 == local",
        "single local alias recognition",
    )
    require(
        gc_hint,
        "fn gc_hint_is_constructor_initialization",
        "field initialization classification",
    )
    require(
        gc_hint,
        "ignore_initialization_receiver",
        "initialization receiver exclusion",
    )

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

    guarded = gc_hint.find("if gc_hint_translation_valid(insts, result.instructions)")
    apply_result = gc_hint.find(
        "MirBlock_set_instructions(block, result.instructions)", guarded
    )
    restore_original = gc_hint.find("MirBlock_set_instructions(block, insts)", guarded)
    if guarded < 0 or apply_result < guarded or restore_original < apply_result:
        raise ValueError("optimized instructions are not guarded by fail-closed validation")

    print("gc-hint-translation-validation: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"gc-hint-translation-validation: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
