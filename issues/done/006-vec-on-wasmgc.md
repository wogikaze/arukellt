---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 6
Track: main
Depends on: 004, 005
Orchestration class: implementation-ready
---

# Vec on WasmGC for T3

## Summary

Replace the current T3 Vec fallback/bridge behavior with a real WasmGC-native representation and operation set.

## Acceptance Criteria

- [x] `docs/current-state.md` no longer needs the limitation that T3 Vec is linear-memory-backed.
- [x] T3 compile/run works for Vec creation, growth, indexing, mutation, and higher-order stdlib operations used by current fixtures.
- [x] Vec element types `i32`, `i64`, `f64`, `String`, and representative aggregate types compile correctly in T3.
- [x] T3 Vec implementation is independent of the T1 bump-allocator model.

## Goal

Finish the biggest missing part of a real T3/WasmGC path: `Vec<T>`.

## Implementation

- Redesign T3 `Vec<T>` in `crates/ark-wasm/src/emit/t3_wasm_gc.rs` around GC struct/array semantics.
- Implement T3 paths for:
  - constructors
  - `push` / `pop`
  - `get` / `get_unchecked` / `set`
  - `len`
  - stdlib higher-order helpers used today (`map`, `filter`, `fold`, `sort`, etc.)
- Handle element-type-dependent lowering for primitive and reference-like types without collapsing back to T1 layout assumptions.
- Make growth/reallocation behavior explicit under the GC model.

## Dependencies

- Issues 004 and 005.

## Impact

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs`
- stdlib Vec fixtures
- possibly MIR aggregate handling for Vec-specific lowering support

## Tests

- Vec fixture matrix across primitive and reference element types.
- Large-growth/reallocation tests.
- Vec-heavy benchmark smoke.

## Docs updates

- `docs/current-state.md`
- `docs/platform/wasm-features.md`
- stdlib docs for Vec operations

## Compatibility

- T3 backend representation changes significantly.
- User-visible Vec semantics must remain unchanged.

## Notes

- This issue is exit-blocking because current docs explicitly record T3 Vec as still linear-memory-backed.