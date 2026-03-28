# Arukellt — Current State

> This document reflects the actual, verified state of the project.
> Current-first source of truth for user-visible behavior and verification gates.
> Updated: 2026-03-28.

## Pipeline

- Current implementation path: `Lexer → Parser → Resolver → TypeChecker → MIR → Wasm`
- Component path (v2): `... → MIR → WasmEmit → WIT generation → wasm-tools component embed/new`
- Refactor target owned by this branch: `Lex → Parse → Bind → Load → Analyze → Resolve → Check+BuildCoreHIR → LowerToMIR → MIRValidate → MIROptimize → BackendPlan → WasmEmit / LLVMEmit → BackendValidate`
- Shared orchestration entry point: `ark-driver::Session`
- Hidden developer dump support exists via `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`

## Targets

| Target | Tier | Status | Run | Notes |
|--------|------|--------|-----|-------|
| wasm32-wasi-p1 | T1 | ✅ Implemented | ✅ | Legacy compatibility path (non-GC environments) |
| wasm32-freestanding | T2 | ❌ Not implemented | ❌ | Registry only |
| wasm32-wasi-p2 | T3 | ✅ Implemented | ✅ | **Primary path** — fully GC-native |
| native | T4 | ❌ Not implemented | ❌ | Requires LLVM 18 |
| wasm32-wasi-p3 | T5 | ❌ Not implemented | ❌ | Future |

## Test Health

- Unit tests: current count is verified by `cargo test --workspace --exclude ark-llvm`
- Fixture harness: **351 passed, 0 failed** (manifest-driven, includes 5 component-compile fixtures)
- Fixture manifest: 351 entries
- Wasm validation is a hard error (W0004)
- Verification entry point: `bash scripts/verify-harness.sh` — **16/16 checks pass**

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

### Binary Size (T1 vs T3 GC-native)

| Source | T1 size | T3 size | Reduction |
|--------|---------|---------|-----------|
| hello.ark | 12,229 B | 954 B | 92% |
| vec.ark | 13,261 B | 1,866 B | 86% |
| closure.ark | 12,222 B | 995 B | 92% |

## Diagnostics and Validation

- Canonical diagnostics registry lives in `crates/ark-diagnostics`
- Diagnostics are tracked by code, severity, and phase origin
- `W0001`: same-body heuristic warning for shared mutable aliasing
- `W0002`: deprecated target alias warning
- `W0004`: generated Wasm failed backend validation — hard error in T3 path
- `W0005`: non-exportable function skipped from component exports
- Structured diagnostic snapshots are available for tests/docs via `ARUKELLT_DUMP_DIAGNOSTICS=1`

## Recent Changes (GC-native track, 2026-03-27)

- **GC-native T3 emitter complete** — all 346 fixtures pass with pure GC output
- Generics via `anyref` polymorphism — `Option<T>`, `Result<T,E>`, `Vec<T>`, generic fns
- `__tupleN_any` structs for generic tuple returns with `ref.i31` boxing/unboxing
- `HashMap<i32,i32>` GC struct with array-backed linear scan
- WASI file I/O (`fs_read_file`, `fs_write_file`) via bridge to linear memory
- `parse_i64`, `parse_f64` returning `Result<i64,String>`, `Result<f64,String>`
- HOF (filter/map/fold/any/find) using `call_indirect` for function dispatch
- Clippy clean, `cargo fmt` clean, `verify-harness.sh` 16/16

## V1 Exit Status: **COMPLETE**

All v1 exit criteria are satisfied as of 2026-03-27.

1. ✅ **T3 compile/run correctness**: `--target wasm32-wasi-p2` compiles and runs all 346
   fixture categories using the fully GC-native T3 backend.
2. ✅ **True GC-native data model**: All values live in Wasm GC heap. No bridge mode.
   Linear memory is used only for WASI I/O byte marshaling.
3. ✅ **T1 retained as compatibility path**: `wasm32-wasi-p1` remains functional for
   non-GC environments (AtCoder/iwasm) but is no longer the primary path.
4. ✅ **Runtime model**: `RuntimeModel::T3WasmGcP2` is the sole T3 runtime model.
   `RuntimeModel::T3FallbackToT1` has been removed.

### What is NOT in scope (post-v1)

- ~~`--emit component` (Component Model output)~~ **→ Implemented in v2**
- ~~WIT generation as a deployment artifact~~ **→ Implemented in v2**
- T4 (native/LLVM) completion — scaffold only, not a gate
- WASI Preview 3 / async-first runtime — future work (T5)
- `call_ref`-based HOF dispatch — current: `call_indirect`; future migration planned

### Completed issue queue (28 issues total)

Issues 001–027 are all in `issues/done/`. The open queue contains only
auto-generated index files (`index.md`, `dependency-graph.md`).

## V2 Exit Status: **COMPLETE**

v2 (Component Model) implementation is complete as of 2026-03-28.

1. ✅ **Component emit**: `--emit component` produces valid `.component.wasm` binaries
   for all scalar, boolean, float, and integer export signatures.
2. ✅ **WIT generation**: `--emit wit` generates correct WIT from `pub fn` signatures.
   Stdlib functions are correctly filtered from the export surface.
3. ✅ **Import parsing**: WIT parser supports interface/function/type import declarations.
   Canonical ABI classification (flat/lower/lift) is implemented.
4. ✅ **Export surface**: Only user `pub fn` with WIT-compatible types are exported.
   Non-exportable functions emit W0005 warnings.
5. ✅ **Resource types**: `own<T>`, `borrow<T>`, `resource` WIT parsing and handle table
   planning implemented.
6. ✅ **CLI integration**: `--wit <path>` flag, `--emit component`, `--emit all` all work.
7. ✅ **No v1 regressions**: All existing 351+ fixture tests continue to pass.
8. ✅ **Documentation**: ADR-008 (component wrapping), migration guide (v1→v2), ABI docs.

### V2 issues (028–035)

Issues 028–035 are the v2 Component Model track.

## Known Limitations

- `--emit component` requires external `wasm-tools` binary and WASI adapter module
- `--deny-clock` and `--deny-random` are not enforced (hard error)
- No `--dir` flag = no filesystem access (deny-by-default)
- HashMap only supports `<i32,i32>` monomorphization in T3; other key/value types use stubs
- HOF dispatch uses `call_indirect` + function table (not `call_ref`); requires table section
- `heap_ptr` global retained for I/O buffer management and legacy `VecLiteral` fallback
- `ark-llvm` is excluded from default builds (requires LLVM 18)
- Component Model: string/list canonical ABI lift/lower not yet wired into emitter
- Component Model: async features (streams, futures) not yet supported

## API Baseline Notes

- `parse_i64` baseline shape: `Result<i64, String>`
- `parse_f64` baseline shape: `Result<f64, String>`
- The observed outputs are frozen in `tests/baselines/api-baseline.json`
