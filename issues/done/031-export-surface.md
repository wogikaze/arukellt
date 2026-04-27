---
Status: done
Created: 2026-03-28
Updated: 2026-03-30
ID: 25
Track: component-model
Depends on: 029
Orchestration class: implementation-ready
---
# pub fn export surface & WIT export generation
**Blocks v1 exit**: no

## Summary

Define the rules for which Arukellt functions become WIT exports. Generate canonical ABI
export adapter functions that lower GC-native return values and lift GC-native parameters
at the component boundary. Extend `mir_to_wit_world()` to produce complete, validated
WIT world definitions suitable for component wrapping.

## Context

The existing `mir_to_wit_world()` in `crates/ark-wasm/src/component/mod.rs` already extracts
`pub fn` (excluding `__*`, `_start`, `main`) and maps types via `type_to_wit()`. However:

1. There is no mechanism for the user to control which functions are exported (all `pub fn` are
   exported by default).
2. Export adapters (canonical ABI lowering for return values, lifting for parameters) are not
   generated — the existing code only produces WIT text, not Wasm adapter functions.
3. The `WitWorld` struct lacks import declarations.

This issue completes the export pipeline so that component wrapping (#030) has correct
export adapters to work with.

## Acceptance Criteria

- [x] `pub fn` in the source module are exported by default (no annotation needed for v2).
      `main` and `_start` are excluded. Internal functions (`__*`) are excluded.
- [x] For each WIT-exportable function, a canonical ABI export adapter is generated in the
      core Wasm module. The adapter:
      - Lifts canonical ABI parameters (linear memory → GC refs) using the lift functions from #029.
      - Calls the real Arukellt function.
      - Lowers GC-native return values (GC refs → canonical ABI flat values) using lower functions from #029.
- [x] `WitWorld` struct extended with `imports: Vec<WitFunction>` to hold import declarations
      from #028.
- [x] `mir_to_wit_world()` populates both `functions` (exports) and `imports`.
- [x] `generate_wit()` emits `import` declarations in addition to `export` declarations.
- [x] Functions with non-exportable parameter or return types (closures, TypeVar) produce a
      clear compile-time warning (new diagnostic code) instead of silently being skipped.
- [x] Struct and enum names in WIT output use stable kebab-case names derived from the
      Arukellt source name (not internal `struct-{id}` / `enum-{id}` format).
- [x] At least 3 test cases: (a) simple scalar export, (b) struct parameter + string return,
      (c) function with closure param is correctly excluded with warning.

## Key Files

- `crates/ark-wasm/src/component/mod.rs` — extend `mir_to_wit_world()`, WitWorld.imports
- `crates/ark-wasm/src/component/wit.rs` — `generate_wit()` import section, WitWorld struct update
- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — emit export adapter functions
- `crates/ark-diagnostics/src/lib.rs` — new diagnostic code for non-exportable functions

## Notes

- v2 does not introduce `#[export]` or `#[no_export]` annotations. All `pub fn` with
  WIT-compatible signatures are exported. Annotation-based control is a v3 candidate.
- The export adapter function naming convention: `$__cabi_export_{name}` (following the
  canonical ABI naming convention from the Component Model spec).
- Export adapters must handle multi-value returns if the function returns a tuple or struct.
  The canonical ABI flattening rules apply.