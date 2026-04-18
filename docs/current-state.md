# Arukellt — Current State

> This document reflects the actual, verified state of the project.
> Current-first source of truth for user-visible behavior and verification gates.
<!-- BEGIN GENERATED:CURRENT_STATE_UPDATED -->
> Updated: 2026-04-14.
<!-- END GENERATED:CURRENT_STATE_UPDATED -->

## Pipeline

The **corehir** path is the default for all CLI commands (`compile`, `build`, `run`, `check`).
The legacy path remains available as an opt-in fallback via `--mir-select legacy`.

- **corehir** (default): `Lexer → Parser → Resolver → TypeChecker → CoreHIR → MIR → Wasm`
- **legacy** (opt-in fallback): `Lexer → Parser → Resolver → TypeChecker → MIR → Wasm`
- Component path (v2): `... → MIR → WasmEmit → WIT generation → wasm-tools component embed → wasm-tools component new` (default wrap passes `--adapt wasi_snapshot_preview1=…` to `component new` when a Preview 1 adapter module is discoverable; see [target-contract.md](target-contract.md#component-output-separate-guarantee-tier))
- Shared orchestration entry point: `ark-driver::Session`
- Developer dump support: `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`

<!-- BEGIN GENERATED:CURRENT_STATE_TARGETS -->
## Targets

| Target | Tier | ADR-013 Tier | Status | Run | Notes |
|--------|------|--------------|--------|-----|-------|
| `wasm32-wasi-p1` | T1 | supported | stable | Yes | Supported: full fixture coverage, AtCoder/competition target |
| `wasm32-freestanding` | T2 | scaffold | scaffold | No | Scaffold: minimal core Wasm emitter proof and validator pass; no runtime execution support yet |
| `wasm32-wasi-p2` | T3 | primary | stable | Yes | Primary (ADR-013): canonical GC-native path, all CI gates apply |
| `native` | T4 | scaffold | scaffold | No | Scaffold: ark-llvm exists, requires LLVM 18, no test infrastructure |
| `wasm32-wasi-p3` | T5 | not-started | not-started | No | Not started: target id exists but no backend, runtime, or scaffold code |
<!-- END GENERATED:CURRENT_STATE_TARGETS -->

### `wasm32-freestanding` (T2)

`wasm32-freestanding` is **implemented for compile-only** in `crates/ark-target`
(`implemented: true`, `run_supported: false`). The backend scaffold
`crates/ark-wasm/src/emit/t2_freestanding.rs` emits a minimal core Wasm module
(linear memory plus empty `_start`, no WASI imports). Regression proof:
`cargo test -p arukellt --test t2_scaffold` against
`tests/fixtures/t2/t2_scaffold.ark` with `wasmparser` validation. Full target
verification contract and roadmap context: [target-contract.md](target-contract.md)
and [ADR-020 — T2 I/O surface](adr/ADR-020-t2-io-surface.md).

<!-- BEGIN GENERATED:CURRENT_STATE_TEST_HEALTH -->
## Test Health

- Unit tests: current count is verified by `cargo test --workspace --exclude ark-llvm`
- Fixture harness: 641 passed, 0 failed, 28 skipped (manifest-driven)
- Fixture manifest: 733 entries
- Wasm validation is a hard error (W0004)
- Verification entry point: `bash scripts/run/verify-harness.sh (fast local gate; use --full for full local verification)` — **19/19 checks pass**
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
- `E0500`: module requires a different target (e.g. `std::host::sockets` on T1 emits E0500; use `--target wasm32-wasi-p2`) (error, `resolve`)
- `E0501`: symbol not found in module (e.g. `string::nonexistent_fn()` when the function is not exported by the imported module) (error, `typecheck`)
- Structured diagnostic snapshots are available for tests/docs via `ARUKELLT_DUMP_DIAGNOSTICS=1`
<!-- END GENERATED:CURRENT_STATE_DIAGNOSTICS -->

## CLI Command Surface

The `arukellt` binary exposes the following subcommands:

| Command | Description |
|---------|-------------|
| `arukellt compile <file>` | Compile an `.ark` file to Wasm (T1 or T3) |
| `arukellt run <file>` | Compile and run an `.ark` file |
| `arukellt check <file>` | Type-check without compiling |
| `arukellt build` | Build the project in the current directory (requires `ark.toml`) |
| `arukellt fmt [file]` | Format `.ark` source files |
| `arukellt test <file>` | Run test functions in an `.ark` file |
| `arukellt lint <file>` | Run static analysis lints |
| `arukellt targets` | List supported compilation targets |
| `arukellt analyze` | Wasm binary analysis utilities |
| `arukellt init [dir]` | Initialize a new Arukellt project (`--template minimal\|cli\|with-tests\|wasi-host`, `--list-templates`) |
| `arukellt script` | Run scripts defined in `ark.toml` |
| `arukellt doc <symbol>` | Look up stdlib documentation for a symbol or module |
| `arukellt lsp` | Start the Language Server Protocol server |
| `arukellt debug-adapter` | Start the Debug Adapter Protocol server |
| `arukellt compose` | Compose Wasm component binaries |

### `arukellt doc`

Looks up stdlib manifest metadata and displays:
- Function signature (`fn name(params) -> return`)
- Module path (e.g. `std::host::stdio`)
- Stability (`stable`, `provisional`, etc.)
- Target availability (T1 / T3 flags from `availability` block)
- Doc description, examples, errors, and `see_also` when present

Flags: `--json` (machine-readable output), `--target <TARGET>` (show availability warning for specific target), `--all` (show intrinsic entries).

Unknown symbols produce a "Did you mean?" list of fuzzy candidates. Module paths (e.g. `std::host::http`) list all functions in the module.

## Recent Milestones

- **`arukellt doc` subcommand added (issue 456)** — stdlib manifest lookup via `arukellt doc <symbol>`. Supports `--json`, `--target`, and fuzzy-match "did you mean?" for unknown symbols.
- **HTTP T1 linker path confirmed (issue 446)** — `std::host::http` (`http::get`, `http::request`) is now available on both T1 (wasm32-wasi-p1) and T3 (wasm32-wasi-p2) via `register_http_host_fns`. The compile-time E0500 gate was removed from `T3_ONLY_MODULES`; a T3 GC-native match-arm type-inference fix (`detect_specialized_result` propagation for qualified callee paths) enables correct T3 Wasm emission. Error-case fixtures `get_err_dns.ark` and `request_err_refused.ark` pass on both targets.
- **GC-native T3 emitter complete** — the v1 GC-native track closed on 2026-03-27
- **Component / WIT support added in v2** — `--emit component`, `--emit wit`, and `--emit all` are available on `wasm32-wasi-p2`
- **Stdlib v3 track completed** — the stdlib roadmap items tracked as issues 039–059 now live in `issues/done/`
- **T3 runtime correctness sweep (2026-04)** — wasmtime 29.x DRC GC bug mitigated (null collector workaround); peephole local.tee suppressed for GC-ref locals; nested concat scratch-local clobbering fixed; `eq`/`ne`/`split` builtins implemented. Fixture harness now **575/575 pass** with 31 new t3-run entries.
- **Current open queue shifted** — active work now focuses on WASI / `std::host::*` rollout rather than the completed v3 stdlib track
- **`std::host::process::exit` and `abort` available (issue 445)** — `__intrinsic_process_exit(i32)` and `__intrinsic_process_abort()` are wired into the T1 and T3 WASI emitters via `wasi_snapshot_preview1/proc_exit`. Both are noreturn; the emitter emits `unreachable` after every call site. `abort()` uses `proc_exit(134)` (SIGABRT convention). `std::host::process` is no longer a stub.

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
  WIT-imported functions are accepted as callable host imports during component compilation.
4. ✅ **Current export behavior**: non-exportable functions surface `W0005` warnings.
5. ✅ **No regression to core Wasm paths**: T1/T3 core Wasm flows remain available.

### Known v2 carry-over limitations

- `--emit component` / `--emit all` require external `wasm-tools` on `PATH` (also probed under `~/.cargo/bin`). The driver runs `wasm-tools component embed` then `wasm-tools component new`; unless `--wasi-version p2` / `--p2-native` is used, `component new` links the **WASI Preview 1** adapter when a matching adapter module is found (reactor/command `.wasm`, or `ARK_WASI_ADAPTER`).
- **WASI Preview 2 native components** (core Wasm imports `wasi:cli/*` etc. directly, no Preview 1 adapter — [issue 074](../issues/open/074-wasi-p2-native-component.md)) are **deferred to v5+** pending WASI P2 runtime maturity. Today, `--wasi-version p2` only skips the adapter in `wasm-tools` while the T3 core module still uses `wasi_snapshot_preview1` imports (`W0009`; import-table work: [issue 510](../issues/open/510-t3-p2-import-table-switch.md)).
- Component output is still T3-only: use `--target wasm32-wasi-p2` for `--emit component`, `--emit wit`, and `--emit all`
- string/list/complex canonical ABI lift-lower coverage is not complete for every case
- async Component Model features are not supported
- jco browser-facing flow remains blocked upstream (`issues/blocked/037`)

### Component export type tiers

The compiler enforces type-tier restrictions on component exports at compile time:

| Tier | Types | Status | Error |
|------|-------|--------|-------|
| Tier 1 | i32, i64, f32, f64, bool, char, unit enum, scalar record | Supported | — |
| Tier 2 | string, list, option, result | Blocked (canonical ABI lift/lower) | E0401 |
| Tier 3 | resource, stream, future, flags (complex) | Not implemented | E0400/E0402 |

Functions using Tier 2/3 types in exports produce compile errors. Functions with non-exportable
types are excluded from component exports with W0005 warning. Core Wasm binary validation
catches GC reference types that bypass WIT-level checks (W0004).

## Known Limitations

- `--deny-clock` and `--deny-random` are enforced at **compile time** via MIR scan (`mir_uses_capability`). Detection is transitive. These flags apply to the `run` subcommand; the `compile` subcommand does not accept them (compile only emits Wasm bytes, no runtime policy is applied).
- No `--dir` flag means no filesystem access
- `ark-llvm` is excluded from default builds (requires LLVM 18)
- some historical docs remain archived / historical and should not override current-state
- **Host module target-gating**: `std::host::http` is available on all targets (T1 and T3) via the Wasmtime linker (issue 446). `std::host::sockets` remains T3-only (wasm32-wasi-p2); importing it on T1 produces an E0500 compile-time error (issue 448). `std::host::http` uses TCP HTTP/1.1; HTTPS is not supported.

## V4 Optimization Status

The v4 optimization pipeline is fully implemented and active. See [docs/compiler/optimization.md](docs/compiler/optimization.md) for the complete reference.

- **20 MIR passes** implemented in `crates/ark-mir/src/opt/`, running up to 3 fixed-point rounds
- **`--opt-level` 0/1/2** controls which passes run; default is O1 (9 safe passes)
- **Dead function elimination** removes unreachable stdlib functions at O1+ (T1/T2 only — disabled for T3; see below)
- **T3 backend peephole**: `local.set`/`local.get` → `local.tee` conversion at O1+
- **Struct field layout reorder**: hot-field-first layout at O2
- **Backend reachability**: only reachable functions and WASI imports are emitted
- **MIR validation** brackets every pass for early bug detection
- Dump support: `ARUKELLT_DUMP_PHASES=optimized-mir` shows before/after state

### T3 MIR optimization re-enabled (issue #486, 2026-04-15)

Prior to issue #486, T3 (`wasm32-wasi-p2`) was forced to `O0` MIR optimization to
stabilize fixture tests. Issue #486 replaced the blanket override with per-pass gating:

- T3 now runs all 9 O1 MIR passes via `passes::run_all()` (standalone path that bypasses
  `desugar_exprs`, which is not GC-safe)
- Three safe O2 arithmetic passes are also active for T3 at O2: `algebraic_simplify`,
  `strength_reduction`, `string_concat_opt`
- Dead function elimination remains **disabled for T3** — WASI-exported functions that
  are not called from the Arukellt entry point would be incorrectly removed
- Six O2/O3 passes remain gated via `T3_GATED_PASSES` in `session.rs` until each is
  independently verified GC-safe (see `crates/ark-mir/src/passes/README.md`)

The #122 opt-level separation work established the `passes/` directory and the unified
`fn run(module: &mut MirModule, level: OptLevel) -> PassStats` interface that #486 builds on.

## API Baseline Notes

- `parse_i64` baseline shape: `Result<i64, String>`
- `parse_f64` baseline shape: `Result<f64, String>`
- The observed outputs are frozen in `tests/baselines/api-baseline.json`

## Self-Hosting Bootstrap Status

> **Completion criterion:** `scripts/run/verify-bootstrap.sh` exits 0 (no
> SKIP) **and** `scripts/check/check-selfhost-parity.sh` exits 0.
> See [docs/compiler/bootstrap.md](docs/compiler/bootstrap.md) for full details.

Verification status of each bootstrap stage (source: `src/compiler/*.ark`):

| Stage | Description | Status |
|-------|-------------|--------|
| **Stage 0** | Rust compiler compiles all `src/compiler/*.ark` files individually | ✅ **Verified** (CI + `verify-bootstrap.sh --stage1-only`) |
| **Stage 1** | `arukellt-s1.wasm` (selfhost compiler) compiles its own sources | ✅ **Verified** — 9/9 files compile, 377158 bytes |
| **Stage 2** | `sha256(s1) == sha256(s2)` fixpoint | 🔴 **Not reached** — `sha256(s1) ≠ sha256(s2)` (see below) |
| **Fixture parity** | Selfhost compiler passes all harness fixtures | 🟡 **CI scripted** — `check-selfhost-fixture-parity.sh` wired into `--full`; s1.wasm compile errors expected until multi-file module loading is fixed |
| **CLI parity** | Selfhost CLI output matches Rust output for identical inputs | 🔴 **Blocked** — requires fixture parity |
| **Diagnostic parity** | Error messages are identical between Rust and selfhost | 🟡 **CI scripted** — `check-selfhost-diagnostic-parity.sh` wired into `--full`; Rust baseline confirmed; s1.wasm gap expected |

### Fixpoint status

Stage 0 and Stage 1 compile successfully (`stage0-compile: reached`, `stage1-self-compile: reached`).
Fixpoint has **not** been reached:

```
sha256(s1) = a9bdbe3abe1778e8e5c6d30f3d181922c185935050701bafa77f7e363bab0ce3  (377158 bytes)
sha256(s2) = 59fe5d256d065952d75d719eed9c6ba8c2e35bcc9fafdfce06adf28b993964a5  ( 21863 bytes)
```

Root cause (tracked in issue #459 — not fixed in this slice):
- s1.wasm does **not** implement multi-file module loading; `use driver`, `use lexer`, etc. are ignored.
- All cross-module calls are lowered to `i32.const 0` stubs in `emitter.ark`.
- s2.wasm contains only ~24 functions (CLI stubs) vs s1.wasm's ~556 (full compiler).

CI scripts added in issue #459:
- `scripts/check/check-selfhost-fixpoint.sh` — sha256 fixpoint check
- `scripts/check/check-selfhost-fixture-parity.sh` — run fixture output parity
- `scripts/check/check-selfhost-diagnostic-parity.sh` — diagnostic parity

All three are wired into `verify-harness.sh --full` (and individually via `--fixpoint`,
`--selfhost-fixture-parity`, `--selfhost-diag-parity`).  They exit 0 (SKIP) when
`arukellt-s1.wasm` is absent so CI does not hard-fail before bootstrap is built.

### Dual-period policy

The dual-period continues while fixpoint and parity criteria remain unmet.
Both the Rust compiler and the selfhost sources are maintained in parallel.
See [Dual-Period End Condition](docs/compiler/bootstrap.md#dual-period-end-condition)
in bootstrap.md for the exact criteria that close this period.
