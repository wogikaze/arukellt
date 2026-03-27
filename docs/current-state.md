# Arukellt — Current State

> This document reflects the actual, verified state of the project.
> All claims are backed by CI output. Updated: 2026-03-27.

## Pipeline

- Lexer → Parser → Resolver → TypeChecker → MIR → Wasm (T1)
- Shared analysis via `ark-driver::Session`

## Targets

| Target | Tier | Status | Run | Notes |
|--------|------|--------|-----|-------|
| wasm32-wasi-p1 | T1 | ✅ Implemented | ✅ | Default target |
| wasm32-freestanding | T2 | ❌ Not implemented | ❌ | |
| wasm32-wasi-p2 | T3 | ⚠️ Experimental | ✅ | Uses P1 runtime internally |
| native | T4 | ❌ Not implemented | ❌ | Requires LLVM 18 |
| wasm32-wasi-p3 | T5 | ❌ Not implemented | ❌ | Future |

## Test Health

- Unit tests: 95 passed
- Fixture harness: 182 passed, 0 failed
- Fixture manifest: 182 entries

## Recent Changes (Phase 1)

- DRIVER-01: ark-driver crate with Session API
- WASM-01: Mandatory wasmparser validation after emit
- VERIFY-01: Manifest-driven fixture harness (182 fixtures)
- SURFACE-01: Contract shrinkage (component/deny-*/dir defaults)

## Known Limitations

- `--emit component` is not implemented (hard error)
- `--deny-clock` and `--deny-random` are not enforced (hard error)
- No `--dir` flag = no filesystem access (deny-by-default)
- Wasm validation is warning-only (W0004), to be promoted to error
- T3 target uses WASI P1 runtime internally
- ark-llvm excluded from default builds (requires LLVM 18)
