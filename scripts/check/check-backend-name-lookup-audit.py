#!/usr/bin/env python3
"""Enforce an explicit MIR-to-Wasm GC layout boundary."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMPILER = ROOT / "src" / "compiler"
WASM = COMPILER / "wasm"
PLAN = COMPILER / "mir" / "gc_layout_plan.ark"
PLAN_VALIDATOR = COMPILER / "mir" / "gc_layout_plan_validator.ark"
PLAN_BUILDER = COMPILER / "mir" / "lower" / "gc_layout_plan_build.ark"
LOWER_ENTRY = COMPILER / "mir" / "lower" / "entry.ark"
TABLE = WASM / "gc_layout_table.ark"
TABLE_BUILD = WASM / "gc_layout_table_build.ark"
LOOKUP = WASM / "ctx_gc_layout_lookup.ark"
SECTIONS_TYPES = WASM / "sections_types.ark"
AUDIT = WASM / "gc_layout_audit.ark"
POLICY = ROOT / "release" / "proof-policy.json"

FORBIDDEN_BACKEND_TOKENS = (
    "gc_layout_table_ref_offset_for_type_name",
    "gc_layout_table_populate_from_type_table",
    "gc_layout_table_register_struct_names",
    "gc_layout_table_register_enum_variant_names",
    "wasm_ref_type_idx_for_type_name",
    "wasm_ref_type_idx_for_named_gc_struct",
    "gc_layout_type_id_for_name",
    "gc_layout_table_observe_name_lookup",
    "gc_layout_table_observe_fallback_lookup",
    "name_lookup_count: i32",
    "fallback_lookup_count: i32",
)


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def main() -> int:
    plan = PLAN.read_text(encoding="utf-8")
    validator = PLAN_VALIDATOR.read_text(encoding="utf-8")
    builder = PLAN_BUILDER.read_text(encoding="utf-8")
    lower_entry = LOWER_ENTRY.read_text(encoding="utf-8")
    table = TABLE.read_text(encoding="utf-8")
    table_build = TABLE_BUILD.read_text(encoding="utf-8")
    lookup = LOOKUP.read_text(encoding="utf-8")
    sections_types = SECTIONS_TYPES.read_text(encoding="utf-8")
    audit = AUDIT.read_text(encoding="utf-8")
    policy = POLICY.read_text(encoding="utf-8")

    require(plan, "struct MirGcLayoutPlan", "versioned MIR GC layout plan")
    require(plan, "schema_version: i32", "layout plan schema version")
    require(plan, "schema_version: 1", "layout plan v1 construction")
    require(plan, "type_id: TypeId", "explicit semantic TypeId binding")
    require(plan, "source_kind: i32", "explicit layout source kind")
    require(plan, "source_value: i32", "explicit layout source value")
    require(
        validator,
        "fn mir_gc_layout_plan_valid",
        "independent layout plan validator",
    )
    require(
        validator,
        "mir_gc_layout_plan_schema_version(plan) != 1",
        "layout plan version validation",
    )
    require(
        builder,
        "fn mir_module_build_gc_layout_plan",
        "pre-backend layout-plan construction",
    )
    require(
        builder,
        "gc_layout_plan_validator::mir_gc_layout_plan_valid(plan)",
        "fail-closed layout-plan validation",
    )
    require(
        lower_entry,
        "mir_module_build_gc_layout_plan(module, clone(input.target))",
        "layout plan freeze after typed MIR synchronization",
    )
    sync_pos = lower_entry.find("mir_module_sync_all_value_types(module)")
    plan_pos = lower_entry.rfind("mir_module_build_gc_layout_plan(module")
    if sync_pos < 0 or plan_pos < sync_pos:
        raise ValueError("GC layout plan must be frozen after typed-MIR synchronization")

    require(table, "typed_lookup_count: i32", "typed lookup counter")
    require(
        table_build,
        "MirModule_gc_layout_plan(mir)",
        "backend consumption of explicit MIR plan",
    )
    require(
        table_build,
        "mir_gc_layout_binding_type_id(binding)",
        "backend TypeId binding consumption",
    )
    if "corehir::type_table" in table_build or "type_entry_name" in table_build:
        raise ValueError("backend layout table build still inspects semantic names")
    if "MirModule_type_table" in sections_types:
        raise ValueError("Wasm type-section path still passes TypeTable into layout build")

    require(
        lookup,
        "SelfEmitCtx_wasm_ref_type_idx_for_type_id",
        "TypeId lookup boundary",
    )
    require(lookup, "MirLocal_value_type", "typed MIR local lookup")
    require(
        lookup,
        "SelfEmitCtx_wasm_ref_type_idx_for_type_id(\n        ctx,\n        mir_value_type::mir_value_type_type_id(mvt)",
        "storage lookup by explicit TypeId",
    )

    for path in sorted(WASM.rglob("*.ark")):
        text = path.read_text(encoding="utf-8")
        for token in FORBIDDEN_BACKEND_TOKENS:
            if token in text:
                raise ValueError(
                    f"{path.relative_to(ROOT)}: backend layout inference remains: {token}"
                )

    require(audit, '" name=0 fallback=0 conflict="', "zero legacy audit fields")
    if "let name_count" in audit or "let fallback_count" in audit:
        raise ValueError("legacy name/fallback counters remain in backend audit")
    require(
        policy,
        '"explicit_backend_type_abi_layout": true',
        "proof policy backend-layout hard gate",
    )

    print(
        "backend-typeid-layout: PASS: "
        "typed MIR -> validated layout plan v1 -> TypeId-only Wasm registration"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"backend-typeid-layout: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
