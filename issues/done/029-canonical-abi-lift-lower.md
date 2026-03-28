# Canonical ABI lift/lower for GC-native types

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 029
**Depends on**: 028
**Track**: component-model
**Blocks v1 exit**: no

## Summary

Implement the canonical ABI lifting and lowering functions that convert between Arukellt's
GC-native Wasm representations and the Component Model's flat linear-memory representations.
This is the core type translation layer required for component boundary crossings.

## Context

Arukellt v1 stores all aggregate data in Wasm GC heap objects:

| Arukellt type | GC representation |
|---------------|-------------------|
| `String` | `(ref null (array (mut i8)))` |
| `Vec<T>` | `(ref null (struct (mut (ref $arr_T)) (mut i32)))` |
| `struct Foo` | `(ref null (struct (field ...)))` |
| `enum Bar` | subtype hierarchy with `br_on_cast` |
| `Option<T>` | subtype of enum base |
| `Result<T, E>` | subtype of enum base |

The Component Model canonical ABI requires a different representation at component boundaries:

| WIT type | Canonical ABI representation |
|----------|------------------------------|
| `string` | `(i32 ptr, i32 len)` in linear memory, UTF-8 |
| `list<T>` | `(i32 ptr, i32 len)` in linear memory |
| `record` | flattened scalar fields |
| `variant` | `(i32 discriminant, payload...)` |
| `option<T>` | `(i32 discriminant, T-or-zero)` |
| `result<T, E>` | `(i32 discriminant, payload)` |

The lift/lower layer must bridge these two representations.

## Acceptance Criteria

- [ ] A `canonical_abi` module exists at `crates/ark-wasm/src/component/canonical_abi.rs`
      containing `lower_*` and `lift_*` functions for each bridged type.
- [ ] **String lower**: `(ref (array (mut i8)))` → copy bytes to linear memory → return `(i32, i32)`.
- [ ] **String lift**: `(i32 ptr, i32 len)` from linear memory → `array.new_data` or byte-copy
      into GC array → return `(ref (array (mut i8)))`.
- [ ] **List lower**: `(ref $vec_T)` → extract array ref + len → copy elements to linear memory
      → return `(i32, i32)`. Element lowering recurses for nested types.
- [ ] **List lift**: `(i32, i32)` → allocate GC array + struct → copy elements from linear memory
      → return `(ref $vec_T)`.
- [ ] **Record lower/lift**: flatten struct fields to/from canonical scalar sequence.
- [ ] **Variant lower/lift**: discriminant + payload to/from canonical flat representation.
      Covers `option<T>` and `result<T, E>` as variant specializations.
- [ ] **Scalar pass-through**: `i32`, `i64`, `f32`, `f64`, `bool`, `char` require no conversion
      (validated by tests).
- [ ] Linear memory allocation strategy for canonical ABI buffers is defined: reuse the existing
      1-page I/O bridge region (offset `DATA_START=256`) with a bump-style sub-allocator that
      resets per call. Document the memory budget constraint (64KB - 256 = 65280 bytes max per
      component call).
- [ ] Unit tests for each lift/lower pair with round-trip verification (lower then lift = identity).
- [ ] The `GcTypeRegistry` in `t3_wasm_gc.rs` is extended if needed to generate helper functions
      for lift/lower (e.g., `$__canonical_lower_string`, `$__canonical_lift_string`).

## Key Files

- `crates/ark-wasm/src/component/canonical_abi.rs` — new file, lift/lower implementations
- `crates/ark-wasm/src/component/mod.rs` — re-export canonical_abi
- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — emit canonical ABI adapter functions
- `crates/ark-target/src/plan.rs` — may need `AbiClass::CanonicalAbi` or similar

## Notes

- The 1-page (64KB) linear memory constraint means large strings or lists will fail at the
  component boundary. This is an accepted v2 limitation. v3 can introduce `memory.grow` if
  needed. Document the limit clearly.
- Lift/lower functions are emitted as internal Wasm functions (`$__canonical_*`) in the core
  module. They are not exposed as exports.
- The canonical ABI spec version targeted is the one implemented by wasmtime's component
  model support (as of early 2026).
