# GC-native scaffolding: type registry, remove bump allocator

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 019
**Depends on**: none
**Track**: gc-native
**Blocks v1 exit**: no

## Summary

Refactor t3_wasm_gc.rs infrastructure to prepare for GC-native codegen. Build
the GcTypeRegistry, remove the bump allocator / heap_ptr global, fix memory
to 1 page (I/O only), remove Table/Elem sections, and update function
signatures so heap types use `(ref $T)` instead of `i32`.

This phase intentionally breaks compilation of heap-type fixtures — only
scalar-only fixtures should pass after this phase.

## Context

ADR-002 mandated "Wasm GC 前提を採用" but the T3 emitter implements bridge mode
(linear memory + i32 pointers). This issue begins the systematic rewrite.

See plan.md §Type Mapping and §Module Structure for the definitive design.

## Acceptance Criteria

- [ ] `GcTypeRegistry` struct exists with mappings for: `$string` (array mut i8),
      `$arr_i32/$arr_i64/$arr_f64/$arr_string`, `$vec_i32/$vec_i64/$vec_f64/$vec_string`,
      user structs (from `type_table.struct_defs`), enum hierarchies (from `type_table.enum_defs`
      — supertype + variant subtypes using `sub`).
- [ ] `heap_ptr` global (Global 0) is removed. `GlobalSection` is empty or absent.
- [ ] Memory is fixed at `(memory 1 1)` — 1 page, not growable. Used only for WASI IOV.
- [ ] `TableSection` and `ElementSection` are removed (call_ref replaces call_indirect).
- [ ] `type_to_val()` returns `ValType::Ref(RefType { ... HeapType::Concrete(idx) })` for
      String, Struct, Enum, Vec, Option, Result types.
- [ ] Function signatures in the type section use ref types for heap-typed params/returns.
- [ ] Data segments still hold string literal bytes (consumed later by `array.new_data`).
- [ ] `cargo build --workspace --exclude ark-llvm --exclude ark-lsp` succeeds.
- [ ] Scalar-only fixtures (control/, operators/) still compile via `t3-compile`.

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — main target
- `crates/ark-wasm/src/emit/mod.rs` — dispatch and validation

## Notes

- The existing `TypeAlloc` helper already knows how to create StructType/ArrayType —
  extend it into the full `GcTypeRegistry`.
- Enum subtypes: register with `SubType { is_final: false, supertype_idx: None }` for
  base and `SubType { is_final: true, supertype_idx: Some(base_idx) }` for variants.
- Data segments change semantic meaning: offsets no longer matter for linear memory
  placement — they become `dataidx` references for `array.new_data`.
