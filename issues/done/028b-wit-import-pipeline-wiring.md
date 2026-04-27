---
Status: done
Created: 2026-04-15
Updated: 2026-04-15
ID: 028b
Track: component-model
Depends on: none
Orchestration class: implementation-ready
---
# WIT import pipeline wiring — flags, resolver injection, pipeline integration
**Blocks v1 exit**: no

## Summary

Three acceptance items from issue #028 are NOT implemented. The 2026-04-15 audit confirmed
the WIT parser is complete but the import pipeline is unwired. This issue tracks the remaining
~130 lines of work to close #028.

## Parent extraction note — 2026-04-15

This issue was extracted from #028 as the concrete implementation follow-up for the
remaining gaps. Completing #028b does not automatically close #028; the parent issue
must still pass evidence review against its corrected acceptance criteria.

## Partial slice note — 2026-04-15

Wave 1 implemented the `flags` / `E0090` slice in the worktree but did **not** complete
the issue:
- required verification was blocked by unrelated compile failures in
  `crates/ark-wasm/src/emit/t3_wasm_gc/mod.rs`
- the slice was not committed

Treat #028b as still open and dispatch follow-up only after the blocking worktree state is
cleared and the slice can be verified + committed.

## Missing items (from #028 audit)

### 1. `WitType::Flags` + E0090 diagnostic (~35 lines)

- Add `Flags(Vec<String>)` variant to `WitType` enum in `crates/ark-wasm/src/component/wit.rs`
- Add `to_wit()` formatting for `Flags`
- Add `flags { name, ... }` parsing in `crates/ark-wasm/src/component/wit_parse.rs`
  (follows the same `{` / identifier / `,` / `}` pattern as `parse_enum`)
- Emit an E0090 diagnostic when a function whose parameter or return type contains
  `WitType::Flags` is lowered at codegen time (not a panic — a graceful compile error)
- Add a parse test and a diagnostic-fires test

### 2. Extern function registration in resolver (~65 lines)

- Add `ExternWitFn` variant to `SymbolKind` in `crates/ark-resolve/src/scope.rs`
- Add `inject_wit_externs(table: &mut SymbolTable, scope: ScopeId, externs: &[(&str, WitFnSig)])` in `crates/ark-resolve/src/resolve.rs` (or a new `wit.rs` file under `ark-resolve/src/`)
- Add `wit_files: Vec<PathBuf>` field to `Session` in `crates/ark-driver/src/session.rs`
- In `Session::compile_component_with_world` (and `compile_with_entry`), parse each `wit_file`,
  call `wit_interface_to_mir_imports`, populate `MirModule.imports`, and call `inject_wit_externs`
  before type-check so WIT-imported names resolve
- In `crates/arukellt/src/commands.rs`, pass `wit_files` into the session (the flag is already
  accepted and validated; it just needs wiring: 5 lines)

### 3. Full round-trip test through the pipeline (~30 lines)

- Add a test that: creates a temp `.wit` file, compiles an `.ark` source file that calls a
  function from that WIT interface, and asserts `MirModule.imports` contains the expected entry
- Can live in `tests/fixtures/component/` or as an integration test in `crates/ark-driver/`

## Key files

- `crates/ark-wasm/src/component/wit.rs` — add `Flags` variant to `WitType`
- `crates/ark-wasm/src/component/wit_parse.rs` — add `flags` parsing + E0090 check
- `crates/ark-resolve/src/scope.rs` — add `ExternWitFn` symbol kind
- `crates/ark-resolve/src/resolve.rs` — add extern injection function
- `crates/ark-driver/src/session.rs` — add `wit_files`, populate `MirModule.imports`
- `crates/arukellt/src/commands.rs` — wire `wit_files` into session (already in scope)

## Completion criteria

- [x] `WitType::Flags` variant exists and `flags { ... }` blocks are parsed
- [x] E0090 diagnostic fires (no panic) when a WIT function with flags type is lowered
- [x] `ark-resolve` has a public `inject_wit_externs` entry point
- [x] Session parses `--wit` files and injects externs before type-check
- [x] `MirModule.imports` is non-empty after compiling with `--wit`
- [x] Round-trip test: parse WIT → resolver injection → MIR compilation → `imports` verified
- [x] `bash scripts/run/verify-harness.sh --quick` passes
- [x] `cargo test` passes

## Completion note — 2026-04-15

Resolved by commits `93d82ee` and `c2d2fd7`.
The WIT import pipeline is now wired end-to-end: `--wit` files are parsed,
resolver externs are injected before type-check, `MirModule.imports` is
populated during compilation, and unsupported `flags` types fail gracefully via
E0090 diagnostics.

## Notes

- Do NOT close #028 until all three items above are complete.
- Flags parsing is the smallest slice (~35 lines) and can be done independently.
- The resolver wiring depends on knowing the WIT function names so the type-checker accepts calls
  to them; the exact shape of `ExternWitFn` in `SymbolKind` should be kept minimal (name only,
  no full type signature in the symbol table for v2).