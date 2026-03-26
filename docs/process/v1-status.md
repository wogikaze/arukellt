# v1 Implementation Status

> **Last updated**: 2026-06-03
> **Branch**: `feature/arukellt-v1`
> **Test results**: 95 unit tests pass, 170/175 fixture tests pass (5 module helpers, not standalone)

This document tracks the implementation status of v1 features. For v0 baseline status,
see [v0-status.md](v0-status.md).

## Language Features (Category A) — ALL COMPLETE ✅

| Feature | Status | Commit | Notes |
|---------|--------|--------|-------|
| A1: Closure capture (reference types) | ✅ runnable | cdcc794 | Captures by reference, `\|x\| expr` syntax |
| A2: Enum struct variants | ✅ runnable | cdcc794 | `Variant { field: Type }` syntax |
| A3: Match guard (`if` guard) | ✅ runnable | e3627f4 | `Pattern if cond => ...` |
| A4: Or-pattern | ✅ runnable | e3627f4 | `1 \| 2 \| 3 => ...` |
| A5: Struct pattern matching | ✅ runnable | e3627f4 | `Point { x, y } => ...` |
| A6: Iterator trait + generic for | ✅ runnable | b42341a | `for item in values(v)`, `for i in 0..n` |
| A7: Display trait + f"..." | ✅ runnable | cdcc794 | Custom type interpolation in f-strings |
| A8: ? operator From trait | ✅ runnable | b42341a | Auto-conversion between error types |
| A9: User-defined generic struct | ✅ runnable | e3627f4 | `struct Pair<T, U> { ... }` |
| A10: Trait bounds | ✅ runnable | e3627f4 | `fn foo<T: Display>(x: T)` |

## Standard Library (Category B) — ALL COMPLETE ✅

| Feature | Status | Notes |
|---------|--------|-------|
| B1: parse_i64 / parse_f64 | ✅ runnable | Returns `Result<i64/f64, String>` |
| B2: sort_i64/f64, map/filter_String | ✅ runnable | Bubble sort, type-specialized HOFs |
| B3: HashMap | ✅ runnable | Type-specialized (i32, String keys) |
| B4: assert / assert_eq | ✅ runnable | Panics with message on failure |
| B5: io/clock / io/random | ✅ runnable | WASI clock_time_get, random_get |

## Modules (Category D) — ALL COMPLETE ✅

| Feature | Status | Notes |
|---------|--------|-------|
| D1: Module name collision detection | ✅ runnable | Detects duplicate exports |
| D2: Circular import error messages | ✅ runnable | Clear error with import chain |

## Diagnostics (Category E) — ALL COMPLETE ✅

| Feature | Status | Notes |
|---------|--------|-------|
| E1: Silent failure elimination | ✅ runnable | All errors produce diagnostics |
| E2: Fix-it hints in error messages | ✅ runnable | Suggestions for common mistakes |

## Toolchain (Category F)

| Feature | Status | Notes |
|---------|--------|-------|
| F1: --target flag | ✅ runnable | 5 canonical targets with alias warnings |
| F2: LLVM IR backend | ⚠️ scaffold | ark-llvm crate: scalars, arithmetic, control flow. No heap types. Requires LLVM 18. |
| F3: LSP server | ⚠️ scaffold | ark-lsp crate: hover, completion, diagnostics. No go-to-definition yet. |

## T3 Wasm GC + WASI p2 Infrastructure — ALL COMPLETE ✅

### Main Track

| Milestone | Status | Notes |
|-----------|--------|-------|
| M1: Target registry | ✅ | ark-target crate, 5 canonical targets |
| M2: Backend IR abstraction | ✅ | Structural split, BackendType descriptors |
| M3: T3 Wasm GC emitter | ⚠️ scaffold | t3_wasm_gc.rs framework, delegates to T1 |
| M4: WIT generation | ✅ | component/wit.rs, --emit wit support |
| M5: WASI capability layer | ✅ | --dir, --deny-fs, --deny-clock, --deny-random |
| M6: T1 compat validation | ✅ | validate_emit_kind, emit restrictions |
| M7: CLI emit surface | ✅ | --emit flag, EmitKind, artifact naming |

### Parallel Track

| Milestone | Status | Notes |
|-----------|--------|-------|
| P1: Bootstrap tasks | ✅ | mise.toml tasks |
| P2: CI matrix | ✅ | integrity + behavior jobs |
| P3: Executable docs | ✅ | 7 examples, all verified |
| P4: Benchmark/parity probe | ✅ | 4 fixtures, parity-check.sh |

### Cross-cutting

| Milestone | Status | Notes |
|-----------|--------|-------|
| X1: Diagnostics | ✅ | E0305-E0307, W0002 codes |
| X2: Docs sync | ✅ | wasm-features.md, ADR-007 |
| X3: Migration strategy | ✅ | docs/migration/t1-to-t3.md |
| X4: Operational policy | ✅ | docs/process/policy.md |

## Phase 2+ (External Dependencies Required)

These items require external tooling or runtime maturity:

| Item | Blocker | Notes |
|------|---------|-------|
| C1/C2: Wasm GC real backend | Wasm GC runtime maturity | T3 scaffold exists, actual GC emission pending |
| C3/C4: Escape analysis | Profiling infrastructure | Requires GC heap measurement tooling |
| F2 full: LLVM heap types | LLVM 18 availability | String, Vec, struct lowering to native |
| F3 full: LSP go-to-def | Incremental parsing | Needs position-to-AST mapping |

## Backend Reality (v1 update)

| Aspect | v0 State | v1 State |
|--------|----------|----------|
| **Primary target** | wasm32-wasi-p1 (T1) | T1 production + T3 scaffold |
| **Memory model** | Bump allocator | Same (T3 GC pending) |
| **WASI version** | p1 only | p1 + p2 capability flags |
| **Targets** | T1 only | T1/T2/T3/T4 registry, T1 production |
| **CLI** | `run` only | `compile`/`run` + `--target`/`--emit` |
| **Emit formats** | wasm only | wasm, wat, wit, all |
| **LLVM** | none | ark-llvm scaffold (feature-gated) |
| **LSP** | none | ark-lsp scaffold (diagnostics, hover, completion) |
