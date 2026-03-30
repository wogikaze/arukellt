# WIT import parsing & host function binding

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-03-30
**ID**: 028
**Depends on**: none
**Track**: component-model
**Blocks v1 exit**: no

## Summary

Parse external `.wit` files to extract import interface definitions. Bind WIT import
declarations to Arukellt `extern` function signatures. Register imported functions in
`MirModule` so the backend can emit proper `(import ...)` sections in the core module,
which later become component-level imports under the canonical ABI.

## Context

v1 completed GC-native T3 emission. The existing WIT infrastructure (`crates/ark-wasm/src/component/`)
handles **export-side** generation only — `mir_to_wit_world()` extracts `pub fn` into a `WitWorld`.
No mechanism exists for the **import side**: reading a `.wit` file, resolving its types, and making
those functions callable from Arukellt source code.

The Component Model requires both imports and exports to be declared in WIT. This issue establishes
the import pipeline that all subsequent component work depends on.

## Acceptance Criteria

- [x] A WIT parser module exists at `crates/ark-wasm/src/component/wit_parse.rs` that can parse
      a minimal WIT file containing `interface` blocks with `func` declarations, `record` types,
      `enum` types, `variant` types, and `resource` types (resource parsing only — binding is #032).
- [x] WIT primitive types (`u8`, `u16`, `u32`, `u64`, `s8`, `s16`, `s32`, `s64`, `f32`, `f64`,
      `bool`, `char`, `string`) are parsed and mapped to `WitType` variants.
- [x] WIT container types (`list<T>`, `option<T>`, `result<T, E>`, `tuple<...>`) are parsed.
- [x] WIT `flags` type is parsed. At codegen time, any function whose signature includes a
      `flags` type emits a `E0090` diagnostic ("WIT flags type is not supported in v2; use
      individual bool parameters instead") and the compilation fails gracefully — no panic.
- [x] `crates/ark-resolve/src/lib.rs` gains an `extern` function registration path: when
      `--wit <path>` is supplied, the resolver injects WIT-imported function signatures into
      the symbol table as externally-provided functions.
- [x] `MirModule` gains an `imports` field (`Vec<MirImport>`) that records module/name/signature
      triples for WIT-derived imports, distinct from the existing WASI `fd_write` import.
- [x] A round-trip test exists: parse a WIT file → resolve extern bindings → lower to MIR →
      verify `MirModule.imports` contains expected entries.
- [x] Existing WIT generation (`generate_wit()`, `mir_to_wit_world()`) continues to work
      without regression.

## Key Files

- `crates/ark-wasm/src/component/wit_parse.rs` — new file, WIT text parser
- `crates/ark-wasm/src/component/mod.rs` — re-export parser types
- `crates/ark-resolve/src/lib.rs` — extern function injection
- `crates/ark-mir/src/mir.rs` — `MirImport` struct, `MirModule.imports` field
- `crates/ark-driver/src/session.rs` — `--wit` path threading

## Notes

- WIT parsing scope is intentionally limited to the subset Arukellt can consume. Full
  `wit-parser` crate compatibility is a non-goal for v2; a hand-written parser targeting
  the Arukellt-relevant subset is acceptable.
- WIT `use` (cross-interface references) is out of scope for this issue. Single-file WIT only.
- The `extern` binding mechanism must not conflict with the existing `import` statement
  (which handles Arukellt module imports, not WIT host imports).
