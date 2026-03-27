# GC-native scalars, control flow, and direct calls

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 020
**Depends on**: 019
**Track**: gc-native
**Blocks v1 exit**: no

## Summary

Verify and fix scalar-type code paths (i32, i64, f64, bool, char) with the
new GC-native scaffolding from 019. Scalars are Wasm value types and should
work identically to bridge mode. Ensure control flow (if/else/loop/block/
break/continue/match on integers), arithmetic, comparison, and direct
function calls all emit correctly.

## Acceptance Criteria

- [ ] All `t3-compile:control/*` fixtures compile successfully.
- [ ] All `t3-compile:operators/*` fixtures compile successfully.
- [ ] All `t3-compile:functions/*` fixtures compile successfully (scalar-only paths).
- [ ] All `t3-compile:variables/*` fixtures compile successfully.
- [ ] All `run:control/*` fixtures pass execution with correct output.
- [ ] All `run:operators/*` fixtures pass execution with correct output.
- [ ] All `run:functions/*` fixtures pass execution with correct output (scalar-only).
- [ ] All `run:variables/*` fixtures pass execution with correct output.
- [ ] `verify-harness.sh --quick` passes for the above fixture categories.

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — emit_operand for scalar ops

## Notes

- This phase is mostly verification — scalar paths should not require
  significant changes from bridge mode since i32/i64/f64 are native Wasm
  value types in both modes.
- The main risk is that removing heap_ptr global shifts global indices and
  breaks GlobalGet/GlobalSet references. Audit all global index usage.
- println for scalars (i32, bool) requires the I/O bridge helper functions
  which write to linear memory. These helpers must be adapted to work with
  the reduced 1-page memory layout.
