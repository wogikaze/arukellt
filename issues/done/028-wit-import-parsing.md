# WIT import parsing & host function binding

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-18
**ID**: 028
**Depends on**: 028b
**Track**: component-model
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: WIT parser exists but is only used in its own tests. CLI --wit flag not threaded into resolver/session/compile pipeline.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Verification audit — 2026-04-15

Checked each acceptance item against the actual codebase. Three items are NOT implemented
despite being marked `[x]`. See corrected criteria below.

**Superseded for import threading — 2026-04-18:** The gap findings below for CLI → session →
resolver/MIR were accurate at the time of that audit; the **Pipeline audit — 2026-04-18** section
records current repo wiring with file/line anchors. Inline “MISSING” notes under acceptance
criteria are historical; the **2026-04-18** lines there summarize present behavior.

## Parent note — 2026-04-15

Child [#028b](../done/028b-wit-import-pipeline-wiring.md) tracked the wiring slice; it is **done**.

## 028 remains the parent acceptance umbrella; closure is evidenced by the 2026-04-18 pipeline

audit table and the updated acceptance notes.

## Pipeline audit — 2026-04-18

`--wit` is defined only on `arukellt compile` (`crates/arukellt/src/main.rs`, `Compile { wit_files }`).
Handlers validate paths and (for `--emit component` / `--emit all`) preflight `flags` → **E0090**
before building a session.

| Step | Where | What happens |
|------|--------|----------------|
| CLI flag | `main.rs` (~`wit_files` on `Compile`) | Paths parsed; passed into `cmd_compile`. |
| Exist + E0090 preflight | `commands.rs` (`cmd_compile`, `preflight_wit_flags_for_component`, `first_flags_diagnostic`) | Missing files → exit 1; `flags` in signatures → rendered **E0090** when emit is component/all. |
| Session storage | `session.rs` (`Session.wit_files`, ~L210–213) | `Vec<PathBuf>` holds import WIT paths for the compile invocation. |
| Assigned from CLI | `commands.rs` | `session.wit_files = wit_files.clone()` for default/core Wasm compile (~L632), component emit (~L587), and the secondary session used for `--emit all` component output (~L708). **Not** set on the `--emit wit`-only branch (export WIT from `.ark` only). |
| Symbol table | `session.rs` `run_frontend_for` (~L661–672) | After `resolve`, `collect_wit_extern_functions` → `ark_resolve::inject_wit_externs` on `resolved.symbols` (`ExternWitFn`). |
| Resolver API | `resolve.rs` `inject_wit_externs` (~L511–537) | Idempotent name registration in global scope. |
| Type-check | `session.rs` `run_frontend_for` (~L691–694) | `TypeChecker::register_extern_function` with params/ret mapped from WIT via `checker_type_from_wit_type`. |
| `MirModule.imports` | `session.rs` `compile_with_entry` (~L976–989) | After MIR lowering, each `wit` file → `parse_wit` → `wit_interface_to_mir_imports` → `mir.imports.extend(...)`. |
| Parse + MIR shaping | `crates/ark-wasm/src/component/**` | `parse_wit`, `wit_interface_to_mir_imports`, `WitType::Flags`; `collect_wit_flags_diagnostics` / component `mod.rs` for structured **E0090** collection. |
| Integration test | `crates/ark-driver/tests/wit_import_roundtrip.rs` | Session with `wit_files` → compile → non-empty `mir.imports`; WIT-backed call compiles through frontend. |

## Summary

Parse external `.wit` files to extract import interface definitions. Bind WIT import
declarations to Arukellt `extern` function signatures. Register imported functions in
`MirModule` so the backend can emit proper `(import ...)` sections in the core module,
which later become component-level imports under the canonical ABI.

## Context

v1 completed GC-native T3 emission. Export-side WIT (`mir_to_wit_world()`, `generate_wit()`) remains;
**import-side** wiring now reads `.wit` paths from the CLI into `Session.wit_files`, injects names
into resolve, registers extern signatures for type-check, and fills `MirModule.imports` during
compilation (see **Pipeline audit — 2026-04-18**).

The Component Model requires both imports and exports to be declared in WIT. This issue tracks the
import pipeline that subsequent component work builds on.

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
      **Audit 2026-04-15**: Claimed missing (historical). **2026-04-18:** `WitType::Flags`,
      `flags { ... }` parsing, CLI preflight + **E0090** (`commands.rs`, `ark-wasm` `collect_wit_flags_diagnostics` / helpers).
- [x] `crates/ark-resolve/src/lib.rs` gains an `extern` function registration path: when
      `--wit <path>` is supplied, the resolver injects WIT-imported function signatures into
      the symbol table as externally-provided functions.
      **Audit 2026-04-15**: Claimed unwired (historical). **2026-04-18:** `Session.wit_files`,
      `inject_wit_externs` + `register_extern_function` in `run_frontend_for`; public API in
      `crates/ark-resolve/src/resolve.rs`.
- [x] `MirModule` gains an `imports` field (`Vec<MirImport>`) that records module/name/signature
      triples for WIT-derived imports, distinct from the existing WASI `fd_write` import.
      **Audit 2026-04-15**: Claimed never populated in real compile (historical). **2026-04-18:**
      `compile_with_entry` extends `mir.imports` from parsed WIT when `wit_files` is non-empty.
- [x] A round-trip test exists: parse a WIT file → resolve extern bindings → lower to MIR →
      verify `MirModule.imports` contains expected entries.
      **Audit 2026-04-15**: Claimed no driver/resolver path (historical). **2026-04-18:**
      `crates/ark-driver/tests/wit_import_roundtrip.rs` exercises session compile + `mir.imports`
      and a WIT-backed call through the frontend.
- [x] Existing WIT generation (`generate_wit()`, `mir_to_wit_world()`) continues to work
      without regression.

## Key Files

- `crates/ark-wasm/src/component/wit_parse.rs` — new file, WIT text parser
- `crates/ark-wasm/src/component/mod.rs` — re-export parser types
- `crates/ark-resolve/src/resolve.rs` — `inject_wit_externs` (re-exported from `lib.rs`)
- `crates/ark-mir/src/mir.rs` — `MirImport` struct, `MirModule.imports` field
- `crates/ark-driver/src/session.rs` — `--wit` path threading

## Notes

- WIT parsing scope is intentionally limited to the subset Arukellt can consume. Full
  `wit-parser` crate compatibility is a non-goal for v2; a hand-written parser targeting
  the Arukellt-relevant subset is acceptable.
- WIT `use` (cross-interface references) is out of scope for this issue. Single-file WIT only.
- The `extern` binding mechanism must not conflict with the existing `import` statement
  (which handles Arukellt module imports, not WIT host imports).

## Completion note — 2026-04-15 (confirmed 2026-04-18)

Resolved by the combined landing of #028b pipeline wiring and the follow-up
`flags`/E0090 diagnostic slice. The import-side WIT pipeline now parses `.wit`
files, injects externs into resolution, records `MirModule.imports` during real
compilation, and rejects unsupported `flags` types with a structured diagnostic.

**2026-04-18:** Repo audit (table above) confirms the CLI → `Session.wit_files` → resolve/type-check
→ `MirModule.imports` path matches corrected acceptance; line anchors are approximate (`~L…`).

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
