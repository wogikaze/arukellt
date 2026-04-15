# WIT import parsing & host function binding

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-15
**ID**: 028
**Depends on**: 028b
**Track**: component-model
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: WIT parser exists but is only used in its own tests. CLI --wit flag not threaded into resolver/session/compile pipeline.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Verification audit — 2026-04-15

Checked each acceptance item against the actual codebase. Three items are NOT implemented
despite being marked `[x]`. See corrected criteria below.

## Parent note — 2026-04-15

The remaining open work from this issue is tracked as focused implementation issue
[#028b](028b-wit-import-pipeline-wiring.md). Treat #028 as the parent acceptance issue:
do not close it until #028b lands and the corrected acceptance items below are proven
by repo evidence.

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
- [ ] WIT `flags` type is parsed. At codegen time, any function whose signature includes a
      `flags` type emits a `E0090` diagnostic ("WIT flags type is not supported in v2; use
      individual bool parameters instead") and the compilation fails gracefully — no panic.
      **Audit 2026-04-15**: `WitType::Flags` variant does not exist; no `flags { ... }` parsing
      in `wit_parse.rs`; no E0090 diagnostic anywhere. MISSING.
- [ ] `crates/ark-resolve/src/lib.rs` gains an `extern` function registration path: when
      `--wit <path>` is supplied, the resolver injects WIT-imported function signatures into
      the symbol table as externally-provided functions.
      **Audit 2026-04-15**: `--wit` files are validated to exist in `commands.rs` but never
      parsed or injected into the resolver symbol table. `ark-resolve` has no extern path.
      Session has no `wit_files` field. MISSING.
- [x] `MirModule` gains an `imports` field (`Vec<MirImport>`) that records module/name/signature
      triples for WIT-derived imports, distinct from the existing WASI `fd_write` import.
      **Audit 2026-04-15**: `MirImport` struct and `MirModule.imports: Vec<MirImport>` exist in
      `crates/ark-mir/src/mir.rs`. However, `imports` is always initialized to `Vec::new()` and
      is never populated during real compilation; only the unit test helper
      `wit_interface_to_mir_imports()` produces `MirImport` values. Struct exists ✅; pipeline
      wiring is part of the extern-registration gap ❌.
- [ ] A round-trip test exists: parse a WIT file → resolve extern bindings → lower to MIR →
      verify `MirModule.imports` contains expected entries.
      **Audit 2026-04-15**: `wit_to_mir_imports_roundtrip` in `wit_parse.rs` only tests
      parse → `MirImport` conversion directly; it does not go through resolver or compiler
      pipeline. `MirModule.imports` is never populated during compilation. MISSING.
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
