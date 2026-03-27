# Arukellt — Current State

> This document reflects the actual, verified state of the project.
> Current-first source of truth for user-visible behavior and verification gates.
> Updated: 2026-03-27.

## Pipeline

- Current implementation path: `Lexer → Parser → Resolver → TypeChecker → MIR → Wasm`
- Refactor target owned by this branch: `Lex → Parse → Bind → Load → Analyze → Resolve → Check+BuildCoreHIR → LowerToMIR → MIRValidate → MIROptimize → BackendPlan → WasmEmit / LLVMEmit → BackendValidate`
- Shared orchestration entry point: `ark-driver::Session`
- Hidden developer dump support exists via `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`

## Targets

| Target | Tier | Status | Run | Notes |
|--------|------|--------|-----|-------|
| wasm32-wasi-p1 | T1 | ✅ Implemented | ✅ | Default target, production path |
| wasm32-freestanding | T2 | ❌ Not implemented | ❌ | Registry only |
| wasm32-wasi-p2 | T3 | ⚠️ Experimental | ✅ | Experimental fallback to T1 runtime internally |
| native | T4 | ❌ Not implemented | ❌ | Requires LLVM 18 |
| wasm32-wasi-p3 | T5 | ❌ Not implemented | ❌ | Future |

## Test Health

- Unit tests: current count is verified by `cargo test --workspace --exclude ark-llvm`
- Fixture harness: 187 passed, 0 failed (manifest-driven)
- Fixture manifest: 187 entries
- Wasm validation is a hard error (W0004)
- Verification entry point: `bash scripts/verify-harness.sh`

## Baseline and Perf Gates

- Baselines are materialized under `tests/baselines/`
- Compile-time baseline cases:
  - `docs/examples/hello.ark`
  - `docs/examples/vec.ark`
  - `docs/examples/closure.ark`
  - `docs/sample/parser.ark`
- Current thresholds:
  - `arukellt check`: median compile time regression must stay within 10%
  - `arukellt compile`: median compile time regression must stay within 20%
- Heavy perf comparisons are intentionally separated from the normal correctness gate

## Diagnostics and Validation

- Canonical diagnostics registry lives in `crates/ark-diagnostics`
- Diagnostics are tracked by code, severity, and phase origin
- `W0001`: same-body heuristic warning for shared mutable aliasing
- `W0002`: deprecated target alias warning
- `W0004`: generated Wasm failed backend validation and is a hard error in this refactor branch
- Structured diagnostic snapshots are available for tests/docs via `ARUKELLT_DUMP_DIAGNOSTICS=1`

## Recent Changes (Quality / Infra Track)

- Baseline collector for perf, fixtures, and stdlib API surface
- Canonical diagnostics registry with phase-aware rendering
- Hidden phase dump support for parse/resolve/mir-oriented snapshots
- Docs consistency checker for fixture count, target status, component emit, and W0004 severity
- Verify harness updated to current manifest-driven fixture execution

## Known Limitations

- `--emit component` is not implemented (hard error)
- `--deny-clock` and `--deny-random` are not enforced (hard error)
- No `--dir` flag = no filesystem access (deny-by-default)
- T3 target still uses the T1/WASI Preview 1 runtime path internally
- T3 `Vec` remains linear-memory-backed in practice
- `ark-llvm` is excluded from default builds (requires LLVM 18)

## API Baseline Notes

- `parse_i64` baseline shape: `Result<i64, String>`
- `parse_f64` baseline shape: `Result<f64, String>`
- The observed outputs are frozen in `tests/baselines/api-baseline.json`
