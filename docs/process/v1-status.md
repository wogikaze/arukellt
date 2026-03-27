# V1 Completion Report

## Status: **V1 EXIT CRITERIA MET**

All v1 exit criteria have been verified and satisfied.

## Verification Results

- **Verify harness**: 16/16 checks passed
- **Unit tests**: 125 passed, 0 failed
- **Fixture harness**: 346 passed, 0 failed, 0 skipped
  - 160 T3-compile fixtures (WasmGC backend)
  - 172 run fixtures (T1 backend)
  - 14 diagnostic fixtures
- **Docs consistency**: OK (346 fixture entries)
- **Clippy**: clean (workspace, -D warnings)
- **Formatting**: clean (cargo fmt --all --check)

## T3 Compile/Run Correctness

T3 (`wasm32-wasi-p2`) compiles all fixture categories:

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
- WASI Preview 2 native imports — T3 uses P1 I/O bridge
- T2 (`wasm32-freestanding`) — planned, not implemented
- T4 (`native`/LLVM) — scaffold only
- T5 (`wasm32-wasi-p3`) — future
- True GC-native data model (current: bridge mode with linear memory)

## Completed Issues

All 18 issues in the v1 exit queue have been completed:

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

## Source of Truth

- Implementation: code in `crates/`
- Current state: `docs/current-state.md`
- Policy: `docs/process/policy.md`
- Migration: `docs/migration/t1-to-t3.md`
