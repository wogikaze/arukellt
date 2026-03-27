# V1 Completion Report

## Status: **V1 EXIT CRITERIA MET — GC-NATIVE**

All v1 exit criteria have been verified and satisfied.
GC-native codegen (T3 emitter) is complete as of 2026-03-27.

## Verification Results

- **Verify harness**: 16/16 checks passed
- **Unit tests**: passed (workspace, excluding ark-llvm)
- **Fixture harness**: 346 passed, 0 failed, 0 skipped
- **Clippy**: clean (workspace, -D warnings)
- **Formatting**: clean (cargo fmt --all --check)

## T3 Compile/Run Correctness

T3 (`wasm32-wasi-p2`) compiles all fixture categories with the fully GC-native emitter:

| Category | Fixtures | Status |
|----------|----------|--------|
| Variables (i32/i64/f64/bool/String) | 13 | ✅ |
| Operators (arithmetic/comparison/logical) | 18 | ✅ |
| Control flow (if/while/loop) | 11 | ✅ |
| Functions (params/returns/recursion) | 14 | ✅ |
| Structs (fields/methods/nested) | 16 | ✅ |
| Enums (variants/match/Option/Result) | 16 | ✅ |
| Closures (captures/HOFs) | 8 | ✅ |
| Vec (push/get/set/pop/len/contains/remove/reverse) | 11 | ✅ |
| String (concat/from/parse) | 10 | ✅ |
| Modules (multi-file/imports) | 10 | ✅ |
| Traits/Generics | 12 | ✅ |
| HOFs (filter/map/fold for i32/i64/f64) | 11 | ✅ |
| Error handling (? operator/Result) | 10 | ✅ |
| HashMap (new/insert/get/contains_key/len) | 1 | ✅ |

## GC-Native Data Model

All value representations use Wasm GC instructions. Linear memory is used
**only** for WASI I/O byte marshaling (1 page, 64 KB).

| Type | GC representation |
|------|-------------------|
| Strings | `(array (mut i8))` — bare GC byte array |
| Vec\<T\> | `(struct (ref $arr_T) i32)` — GC struct + array |
| HashMap\<i32,i32\> | `(struct (ref $arr_i32) (ref $arr_i32) i32)` — array-backed |
| User structs | `(struct (field ...))` — direct GC struct |
| Enums | Subtype hierarchy + `br_on_cast` dispatch |
| Generics | `anyref` polymorphism with `ref.i31` boxing |
| Tuples | `__tupleN_any` (generic) / `__tupleN` (concrete) |
| Closures | Parameter-passing; `call_indirect` for HOF |

## T3 Binary Size Comparison

| Source | T1 size | T3 size | Reduction |
|--------|---------|---------|-----------|
| hello.ark | 12,229 B | 954 B | 92% |
| vec.ark | 13,261 B | 1,866 B | 86% |
| closure.ark | 12,222 B | 995 B | 92% |

## Runtime Model

- `RuntimeModel::T3WasmGcP2` — sole T3 runtime model
- `RuntimeModel::T3FallbackToT1` — removed
- `experimental: false` for `wasm32-wasi-p2` target

## Out of Scope (Post-v1)

- `--emit component` (Component Model output) — hard error
- WASI Preview 2 native imports — T3 uses P1 I/O bridge for fs/clock/random
- T2 (`wasm32-freestanding`) — planned, not implemented
- T4 (`native`/LLVM) — scaffold only
- T5 (`wasm32-wasi-p3`) — future
- `call_ref`-based HOF dispatch — current path uses `call_indirect`
- Full HashMap monomorphization (K/V types beyond i32/i32)

## Completed Issues

All 27 issues in the v1 exit + GC-native queue are in `issues/done/`:

### V1 exit (001–018)
- 001: V1 exit criteria docs
- 002: T3 compile fixture matrix
- 003: Align target profile and backend plan
- 004: Complete T3 emitter compile correctness
- 005: String on WasmGC
- 006: Vec on WasmGC
- 007: Aggregate layouts (struct/enum/match)
- 008: Closure env on WasmGC
- 009: T3 runtime and ABI completion
- 010: Strengthen T3 backend validation
- 011: Promote T3 to primary path
- 012: V1 exit review and completion report
- 013: T3 perf and size telemetry
- 014: T3 diagnostics and phase reporting
- 015: T3 docs current-first sync
- 016: T3 CI and verify policy
- 017: T3 migration and compatibility notes
- 018: LLVM scaffold follow-up

### GC-native codegen (019–027)
- 019: GC-native scaffolding — type registry, subtype hierarchies
- 020: GC-native scalars and control flow
- 021: GC-native structs
- 022: GC-native enums
- 023: GC-native strings
- 024: GC-native Vec + HashMap + generics
- 025: GC-native closures / HOF
- 026: GC-native builtins / WASI I/O
- 027: Full verification, cleanup, ADR update

## Source of Truth

- Implementation: code in `crates/`
- Current state: `docs/current-state.md`
- Architecture decision: `docs/adr/ADR-002-memory-model.md`
- Policy: `docs/process/policy.md`
- Migration: `docs/migration/t1-to-t3.md`
