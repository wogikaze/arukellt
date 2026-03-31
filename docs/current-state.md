# Arukellt — Current State

> This document reflects the actual, verified state of the project.
> Current-first source of truth for user-visible behavior and verification gates.
<!-- BEGIN GENERATED:CURRENT_STATE_UPDATED -->
> Updated: 2026-04-08.
<!-- END GENERATED:CURRENT_STATE_UPDATED -->

## Pipeline

Two lowering paths are available, selected via `--mir-select`:

- **legacy** (default for `compile`): `Lexer → Parser → Resolver → TypeChecker → MIR → Wasm`
- **corehir** (default for `check`): `Lexer → Parser → Resolver → TypeChecker → CoreHIR → MIR → Wasm`
- Component path (v2): `... → MIR → WasmEmit → WIT generation → wasm-tools component embed/new`
- Shared orchestration entry point: `ark-driver::Session`
- Developer dump support: `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`

<!-- BEGIN GENERATED:CURRENT_STATE_TARGETS -->
## Targets

| Target | Tier | ADR-013 Tier | Status | Run | Notes |
|--------|------|--------------|--------|-----|-------|
| `wasm32-wasi-p1` | T1 | supported | stable | Yes | Supported: full fixture coverage, AtCoder/competition target |
| `wasm32-freestanding` | T2 | not-started | unimplemented | No | Not started: no codegen, no tests |
| `wasm32-wasi-p2` | T3 | primary | stable | Yes | Primary (ADR-013): canonical GC-native path, all CI gates apply |
| `native` | T4 | scaffold | scaffold | No | Scaffold: ark-llvm exists, requires LLVM 18, no tests |
| `wasm32-wasi-p3` | T5 | not-started | unimplemented | No | Not started: WASI p3 spec not finalized |
<!-- END GENERATED:CURRENT_STATE_TARGETS -->

<!-- BEGIN GENERATED:CURRENT_STATE_TEST_HEALTH -->
## Test Health

- Unit tests: current count is verified by `cargo test --workspace --exclude ark-llvm`
- Fixture harness: 575 passed, 0 failed, 11 skipped (manifest-driven)
- Fixture manifest: 586 entries
- Wasm validation is a hard error (W0004)
- Verification entry point: `bash scripts/verify-harness.sh (fast local gate; use --full for full local verification)` — **13/13 checks pass**
<!-- END GENERATED:CURRENT_STATE_TEST_HEALTH -->

## GC-Native Data Model (T3, wasm32-wasi-p2)

The T3 emitter is **fully GC-native** as of 2026-03-27. All value representations
use Wasm GC instructions (`struct.new`, `array.new`, `ref.cast`, `br_on_cast`).
Linear memory is retained only for WASI I/O marshaling (1 page, 64 KB).

| Type | Wasm GC representation |
|------|------------------------|
| `i32`, `bool` | `i32` (unboxed); boxed as `(ref i31)` in generic contexts |
| `i64` | `i64` (unboxed) |
| `f64` | `f64` (unboxed) |
| `String` | `(ref null (array (mut i8)))` — bare GC byte array |
| `Vec<T>` | `(ref null (struct (mut (ref $arr_T)) (mut i32)))` |
| `HashMap<i32,i32>` | `(ref null (struct (mut (ref $arr_i32)) (mut (ref $arr_i32)) (mut i32)))` |
| User structs | `(ref null (struct (field ...)))` — direct GC struct |
| Enums / Option / Result | Subtype hierarchy — base + variant subtypes; `br_on_cast` for dispatch |
| Tuples (concrete) | `__tupleN` structs with `i32` fields |
| Tuples (generic) | `__tupleN_any` structs with `anyref` fields; `ref.i31` boxing/unboxing |
| Closures | Parameter-passing captures; `call_indirect` for HOF dispatch |

<!-- BEGIN GENERATED:CURRENT_STATE_PERF -->
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
<!-- END GENERATED:CURRENT_STATE_PERF -->

### Binary Size (T1 vs T3 GC-native)

| Source | T1 size | T3 size | Reduction |
|--------|---------|---------|-----------|
| hello.ark | 12,229 B | 954 B | 92% |
| vec.ark | 13,261 B | 1,866 B | 86% |
| closure.ark | 12,222 B | 995 B | 92% |

<!-- BEGIN GENERATED:CURRENT_STATE_DIAGNOSTICS -->
## Diagnostics and Validation

- Canonical diagnostics registry lives in `crates/ark-diagnostics`
- Diagnostics are tracked by code, severity, and phase origin
- `W0001`: same-body heuristic warning for shared mutable aliasing (warning, `typecheck`)
- `W0002`: deprecated target alias warning (warning, `target`)
- `W0004`: generated Wasm failed backend validation (error, `backend-validate`)
- `W0005`: non-exportable function skipped from component exports (warning, `component`)
- Structured diagnostic snapshots are available for tests/docs via `ARUKELLT_DUMP_DIAGNOSTICS=1`
<!-- END GENERATED:CURRENT_STATE_DIAGNOSTICS -->

## Recent Milestones

- **GC-native T3 emitter complete** — the v1 GC-native track closed on 2026-03-27
- **Component / WIT support added in v2** — `--emit component`, `--emit wit`, and `--emit all` are available on `wasm32-wasi-p2`
- **Stdlib v3 track completed** — the stdlib roadmap items tracked as issues 039–059 now live in `issues/done/`
- **T3 runtime correctness sweep (2026-04)** — wasmtime 29.x DRC GC bug mitigated (null collector workaround); peephole local.tee suppressed for GC-ref locals; nested concat scratch-local clobbering fixed; `eq`/`ne`/`split` builtins implemented. Fixture harness now **575/575 pass** with 31 new t3-run entries.
- **Current open queue shifted** — active work now focuses on WASI / `std::host::*` rollout rather than the completed v3 stdlib track

## V1 Exit Status: **COMPLETE**

All v1 exit criteria are satisfied as of 2026-03-27.

1. ✅ **T3 compile/run correctness**: `--target wasm32-wasi-p2` compiles and runs the v1 exit fixture set using the fully GC-native T3 backend.
2. ✅ **True GC-native data model**: All values live in Wasm GC heap. Linear memory remains only for I/O marshaling.
3. ✅ **T1 retained as compatibility path**: `wasm32-wasi-p1` remains functional for non-GC environments.
4. ✅ **Runtime model**: `RuntimeModel::T3WasmGcP2` is the sole T3 runtime model.

### What is NOT in the original v1 gate

- Component output and WIT generation (added later in v2)
- T4 (native/LLVM) completion
- WASI Preview 3 / async-first runtime work
- `call_ref`-based HOF dispatch migration

## V2 Exit Status: **COMPLETE**

v2 (Component Model) implementation is complete as of 2026-03-28.

1. ✅ **Component emit**: `--emit component` produces `.component.wasm` outputs on the supported `wasm32-wasi-p2` path.
2. ✅ **WIT generation**: `--emit wit` generates WIT for the supported export surface.
3. ✅ **CLI integration**: `--wit <path>`, `--emit component`, and `--emit all` are wired into the CLI.
4. ✅ **Current export behavior**: non-exportable functions surface `W0005` warnings.
5. ✅ **No regression to core Wasm paths**: T1/T3 core Wasm flows remain available.

### Known v2 carry-over limitations

- `--emit component` requires external `wasm-tools` and a WASI adapter module
- string/list/complex canonical ABI lift-lower coverage is not complete for every case
- async Component Model features are not supported
- jco browser-facing flow remains blocked upstream (`issues/blocked/037`)

## Known Limitations

- `--deny-clock` and `--deny-random` are not enforced as full capability filters yet (they are hard-error placeholders)
- No `--dir` flag means no filesystem access
- `ark-llvm` is excluded from default builds (requires LLVM 18)
- some historical docs remain archived / historical and should not override current-state

## V4 Optimization Status

The v4 optimization pipeline is fully implemented and active. See [docs/compiler/optimization.md](docs/compiler/optimization.md) for the complete reference.

- **20 MIR passes** implemented in `crates/ark-mir/src/opt/`, running up to 3 fixed-point rounds
- **`--opt-level` 0/1/2** controls which passes run; default is O1 (9 safe passes)
- **Dead function elimination** removes unreachable stdlib functions at O1+
- **T3 backend peephole**: `local.set`/`local.get` → `local.tee` conversion at O1+
- **Struct field layout reorder**: hot-field-first layout at O2
- **Backend reachability**: only reachable functions and WASI imports are emitted
- **MIR validation** brackets every pass for early bug detection
- Dump support: `ARUKELLT_DUMP_PHASES=optimized-mir` shows before/after state

## API Baseline Notes

- `parse_i64` baseline shape: `Result<i64, String>`
- `parse_f64` baseline shape: `Result<f64, String>`
- The observed outputs are frozen in `tests/baselines/api-baseline.json`
