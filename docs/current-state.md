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
- Shared orchestration entry point: selfhost driver (`src/compiler/driver.ark`); the legacy `crates/ark-driver` Rust crate was removed in #560 (Phase 5).
- Developer dump support: `ARUKELLT_DUMP_PHASES=parse,resolve,corehir,mir,optimized-mir,backend-plan`

<!-- BEGIN GENERATED:CURRENT_STATE_TARGETS -->
## Targets

| Target | Tier | ADR-013 Tier | Status | Run | Notes |
|--------|------|--------------|--------|-----|-------|
| `wasm32-wasi-p1` | T1 | supported | stable | Yes | Supported: full fixture coverage, AtCoder/competition target |
| `wasm32-freestanding` | T2 | scaffold | scaffold | No | Scaffold: minimal core Wasm emitter proof and validator pass; no runtime execution support yet |
| `wasm32-wasi-p2` | T3 | primary | stable | Yes | Primary (ADR-013): canonical GC-native path, all CI gates apply |
| `native` | T4 | scaffold | not-implemented | No | Not implemented: ark-llvm scaffold removed in #586. Future T4 backend will be selfhost-native (#529 Phase 7). |
| `wasm32-wasi-p3` | T5 | not-started | not-started | No | Not started: target id exists but no backend, runtime, or scaffold code |
<!-- END GENERATED:CURRENT_STATE_TARGETS -->

### `wasm32-freestanding` (T2)

`wasm32-freestanding` is **implemented for compile-only** in `crates/ark-target`
(`implemented: true`, `run_supported: false`). The minimal core Wasm scaffold
(linear memory plus empty `_start`, no WASI imports) is emitted by the
selfhost emitter (`src/compiler/emitter.ark`). Regression proof:
`tests/fixtures/t2/t2_scaffold.ark` exercised through the selfhost gates with
`wasmparser` validation. Full target
verification contract and roadmap context: [target-contract.md](target-contract.md)
and [ADR-020 — T2 I/O surface](adr/ADR-020-t2-io-surface.md).

<!-- BEGIN GENERATED:CURRENT_STATE_TEST_HEALTH -->
## Test Health

- Unit tests: current count is verified by `cargo test --workspace`
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

## Performance Snapshot

Current benchmark measurements (target: `wasm32-wasi-p1`, mode: `full`, 5 iterations).
Full results and history are tracked in [`docs/process/benchmark-results.md`](process/benchmark-results.md).

Run benchmarks locally with:

```bash
mise bench            # full measurement (release build)
mise bench:compare    # compare against stored baseline
```

### Benchmark Suite (bench_<suite>_<name>.ark)

| Benchmark | Suite | Compile ms | Run ms | Binary bytes | Correctness |
|-----------|-------|------------|--------|--------------|-------------|
| fib | cpu | ~14 | ~11 | 993 | pass |
| binary_tree | cpu | ~14 | ~19 | 977 | pass |
| vec_ops | cpu | ~21 | ~12 | 1,983 | pass |
| string_concat | cpu | ~14 | ~12 | 1,248 | pass |
| enum_dispatch | cpu | ~15 | ~14 | 1,555 | pass |
| closure_map | cpu | ~15 | ~10 | 2,013 | pass |
| struct_graph | memory | ~18 | ~12 | 1,416 | pass |
| error_chain | compute | ~16 | ~12 | 1,771 | pass |
| parse_tree_distance | parse | ~23 | ~36 | 7,287 | pass |
| http_parser | application | ~14 | ~11 | 1,657 | pass |
| log_processor | application | ~15 | ~12 | 1,906 | pass |
| config_loader | application | ~17 | ~11 | 1,841 | pass |

Source: `tests/baselines/perf/baselines.json` (generated 2026-04-22, wasm32-wasi-p1, selfhost compiler).

Legacy fixtures (`fib`, `binary_tree`, `vec_ops`, `string_concat`) live under `benchmarks/legacy/`
and are retained for cross-language C/Rust comparison. New benchmarks follow the
`bench_<suite>_<name>.ark` naming convention.

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

| Source | T1 size | T3 size | Notes |
|--------|---------|---------|-------|
| hello.ark | 494 B | 494 B | Both targets use same linear-memory emitter at default opt |
| vec.ark | 2,382 B | 2,382 B | Vec ops, same target path |
| closure.ark | n/a | n/a | Compile fails (ICE) — pre-existing, tracked in issue backlog |

Canonical hello.ark sizes at opt-level 2 from [`docs/process/wasm-size-reduction.md`](process/wasm-size-reduction.md): T1=534 B, T3=918 B.

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

- **Selfhost Phase 1 fixpoint achieved** — `sha256(s2) == sha256(s3)` passes (`attainment: reached`). The selfhost compiler (`src/compiler/main.ark`) reproducibly compiles itself. Multi-file module loading, qualified call resolution, and cross-module type handling are all working. See [Self-Hosting Bootstrap Status](#self-hosting-bootstrap-status).
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
- **WASI Preview 2 native components** (core Wasm imports `wasi:cli/*` etc. directly, no Preview 1 adapter — [issue 074](../issues/open/074-wasi-p2-native-component.md)) are **deferred to v5+** pending WASI P2 runtime maturity. Today, `--wasi-version p2` only skips the adapter in `wasm-tools` while the T3 core module still uses `wasi_snapshot_preview1` imports (`W0009`; import-table work: [issue 510](../issues/done/510-t3-p2-import-table-switch.md)).
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
- No `--dir` flag means no filesystem access (module contract: [stdlib/modules/fs.md](stdlib/modules/fs.md))
- T4 (`native`) has no backend: the Rust `ark-llvm` scaffold was removed (#586). Any future native backend will be selfhost-native per #529 Phase 7.
- some historical docs remain archived / historical and should not override current-state
- **Host module target-gating**: `std::host::http` is available on all targets (T1 and T3) via the Wasmtime linker (issue 446). `std::host::sockets` remains T3-only (wasm32-wasi-p2); importing it on T1 produces an E0500 compile-time error (issue 448). `std::host::http` uses TCP HTTP/1.1; HTTPS is not supported.

## V4 Optimization Status

The v4 optimization pipeline is fully implemented and active. See [docs/compiler/optimization.md](docs/compiler/optimization.md) for the complete reference.

- **20 MIR passes** implemented in selfhost `src/compiler/passes/` (the Rust `crates/ark-mir/src/opt/` source was retired in #561 — selfhost is now the source of truth), running up to 3 fixed-point rounds
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
  independently verified GC-safe (see selfhost `src/compiler/passes/README.md`; the
  prior Rust `crates/ark-mir/src/passes/README.md` was retired with the crate in #561)

The #122 opt-level separation work established the `passes/` directory and the unified
`fn run(module: &mut MirModule, level: OptLevel) -> PassStats` interface that #486 builds on.

## API Baseline Notes

- `parse_i64` baseline shape: `Result<i64, String>`
- `parse_f64` baseline shape: `Result<f64, String>`
- The observed outputs are frozen in `tests/baselines/api-baseline.json`

## Self-Hosting Bootstrap Status

> **Completion criterion:** `scripts/run/verify-bootstrap.sh` exits 0 (no
> SKIP) **and** `python scripts/manager.py selfhost parity` exits 0.
> See [docs/compiler/bootstrap.md](docs/compiler/bootstrap.md) for full details.

Verification status of each bootstrap stage (source: `src/compiler/*.ark`):

The selfhost compiler records generic call specializations in the typechecker (`mono_instances`) but does not yet monomorphize or prune them before MIR lowering (see issue #312).

| Stage | Description | Status |
|-------|-------------|--------|
| **Stage 0** | Pinned-reference selfhost wasm (`bootstrap/arukellt-selfhost.wasm`, ADR-029) | ✅ **Committed** — 524 KiB, sha256 `3a035037…f2c` |
| **Stage 2** | Pinned wasm compiles current `src/compiler/main.ark` → `s2.wasm` | ✅ **Verified** |
| **Stage 3** | `sha256(s2) == sha256(s3)` fixpoint (selfhost reproduces itself) | ✅ **Reached** — `attainment: reached` |
| **Fixture parity** | Selfhost compiler passes pinned-vs-current behavioural parity | ✅ **Reached** — 321 PASS, 0 FAIL, 41 SKIP (ADR-029) |
| **CLI parity** | Selfhost `--version` / `--help` match committed snapshot goldens | ✅ **Reached** — 6 PASS, 0 FAIL (ADR-029) |
| **Diagnostic parity** | Selfhost `check` output matches committed `.selfhost.diag` / `.diag` goldens | ✅ **Reached** — 12 PASS, 22 SKIP, 0 FAIL (ADR-029) |

### Fixpoint status

All bootstrap stages pass. The trusted base for verification is the
committed pinned-reference selfhost wasm at
`bootstrap/arukellt-selfhost.wasm` (ADR-029, #585) — the legacy Rust
binary `target/debug/arukellt` is **no longer required** by any selfhost
gate.

The fixpoint criterion is `sha256(s2) == sha256(s3)` — the standard
bootstrap fixpoint where the selfhost compiler reproduces itself from
its own output. Stage 0 is the pinned wasm; Stage 2 is its output on
the current `src/compiler/main.ark`; Stage 3 is Stage 2's output on the
same source.

```
pinned: bootstrap/arukellt-selfhost.wasm
  sha256 = 3a0350371f9dbc37becef03efffa8d20b90827161a0d9fab97163a19de341f2c
  size   = 536 277 bytes
s2/s3 (current):
  sha256 = c16e32efb1b68e1921eb4915e414f554b165d45e299e0c5fd679934e0ba180cc
```

CI checks (`python3 scripts/manager.py selfhost <gate>`) — all four are
selfhost-native per ADR-029:

- `selfhost fixpoint` — pinned-bootstrap + Stage-3 sha256 fixpoint
- `selfhost fixture-parity` — pinned-vs-current execution-output parity across `run:` fixtures
- `selfhost diag-parity` — current selfhost `check` vs committed `.selfhost.diag` / `.diag` goldens
- `selfhost parity --cli` — current selfhost `--version` / `--help` vs `tests/snapshots/selfhost/cli-{version,help}.txt`

All four are wired into `verify-harness.sh --full` (and individually via
`--fixpoint`, `--selfhost-fixture-parity`, `--selfhost-diag-parity`).
They exit 0 (SKIP) when `bootstrap/arukellt-selfhost.wasm` is absent so
CI does not hard-fail on a partial checkout. Refresh policy for the
pinned wasm is documented in `bootstrap/PROVENANCE.md`.

### Dual-period policy

The dual-period continues until fixture and diagnostic parity criteria are fully met.
Both the Rust compiler and the selfhost sources are maintained in parallel.
See [When the Dual Period Ends](docs/compiler/bootstrap.md#when-the-dual-period-ends)
in bootstrap.md for the exact criteria that close this period.

### Selfhost-only execution path (#559, #583, ADR-029)

The user-facing `arukellt` CLI is now invoked through a thin wrapper that runs
the **selfhost wasm exclusively** under `wasmtime`. Per #583 the legacy
`ARUKELLT_USE_RUST=1` opt-in has been **retired** and the `crates/arukellt`
Rust core consumers (`commands.rs`, `cmd_doc.rs`, `native.rs`, `runtime.rs`)
have been deleted; the `arukellt` crate now exists only as a thin wasm-runner
shell that locates the selfhost wasm and execs it under `wasmtime`.

Wrapper artifact: [`scripts/run/arukellt-selfhost.sh`](../scripts/run/arukellt-selfhost.sh).

Resolution order (selfhost wasm only):

1. `$ARUKELLT_SELFHOST_WASM` (explicit override).
2. `.build/selfhost/arukellt-s2.wasm` (fresh build).
3. `.bootstrap-build/arukellt-s2.wasm` (bootstrap intermediate).
4. `bootstrap/arukellt-selfhost.wasm` (committed pinned reference; see
   `bootstrap/PROVENANCE.md`).

If `wasmtime` is unavailable, or no selfhost wasm can be located, the wrapper
hard-fails with a clear diagnostic — there is no longer a Rust fallback.
Setting `ARUKELLT_USE_RUST=1` now exits non-zero with a pointer to this notice.

Examples:

```bash
# Selfhost wasm via wasmtime (the only execution path)
scripts/run/arukellt-selfhost.sh --help
scripts/run/arukellt-selfhost.sh compile docs/examples/hello.ark -o hello.wasm
```

Selfhost gates (`scripts/manager.py selfhost {fixpoint,fixture-parity,parity,diag-parity}`)
are **selfhost-native** per ADR-029 (#585): they bootstrap from the committed
pinned-reference wasm at `bootstrap/arukellt-selfhost.wasm` and never call any
Rust binary. With #583 landed, the `crates/arukellt` shell no longer depends
on `ark-driver`, `ark-mir`, or `ark-stdlib`. Phase 5 has progressed:
`crates/ark-driver` was removed in #560, the legacy Rust Wasm emitter crate
was removed in #562, the T4 LLVM scaffold was removed in #586, and
`crates/ark-mir` was removed in #561. `ark-stdlib` was removed in #563.
Remaining Rust core crate (`ark-typecheck`) is tracked by #564.
