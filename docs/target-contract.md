# Target Verification Contract

This document defines, for each compilation target, exactly which
verification surfaces are **guaranteed**, which are **smoke-tested**,
which exist only as **scaffold**, and which are **not started**.

**Primary target (ADR-013): T3 (wasm32-wasi-p2)** — all CI quality gates
apply to T3 first.  T1 is `supported`.  T2, T4, T5 are `scaffold` or `not-started`.

CI enforces this contract via `ARUKELLT_TARGET` in the `target-behavior`
matrix job.  See `.github/workflows/ci.yml`.

## Status labels

| Label | Meaning |
|-------|---------|
| **guaranteed** | Runs in CI on every push/PR.  Failure blocks merge. |
| **smoke** | Runs in CI but failure is non-blocking, or opt-in flag. |
| **scaffold** | Code exists with targeted proof for shape or validation, but it is not part of the broad target-behavior guarantee tier. |
| **none** | No implementation or test infrastructure exists. |

## Current-state target summary source

`docs/current-state.md` derives its generated target table from this block so the
contract remains the single docs source for target verification status.

<!-- BEGIN GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->
| Target | Tier | ADR-013 Tier | Status | Run | Notes |
|--------|------|--------------|--------|-----|-------|
| `wasm32-wasi-p1` | T1 | supported | stable | Yes | Supported: full fixture coverage, AtCoder/competition target |
| `wasm32-freestanding` | T2 | scaffold | scaffold | No | Scaffold: minimal core Wasm emitter proof and validator pass; no runtime execution support yet |
| `wasm32-wasi-p2` | T3 | primary | stable | Yes | Primary (ADR-013): canonical GC-native path, all CI gates apply |
| `native` | T4 | scaffold | scaffold | No | Scaffold: ark-llvm exists, requires LLVM 18, no test infrastructure |
| `wasm32-wasi-p3` | T5 | not-started | not-started | No | Not started: target id exists but no backend, runtime, or scaffold code |
<!-- END GENERATED:CURRENT_STATE_TARGET_SUMMARY_SOURCE -->

## Target × Verification Surface

### T1 — `wasm32-wasi-p1` (CLI default)

| Surface | Status | Detail |
|---------|--------|--------|
| parse | guaranteed | 342 `run` + 25 `module-run` + 29 `diag` + 8 `module-diag` fixtures |
| typecheck | guaranteed | same fixture set; type errors covered by `diag` entries |
| compile (core Wasm) | guaranteed | all `run`/`module-run` fixtures compile before execution |
| run (wasmtime) | guaranteed | stdout compared against `.expected` files |
| emit component | n/a | component output is T3-only (`wasm32-wasi-p2`) |
| emit WIT | n/a | WIT generation is T3-only |
| host capabilities | guaranteed | `--deny-clock`, `--deny-random` hard-error placeholders |
| determinism | smoke | `tests/baselines/fixture-baseline.json` spot-checked |
| validator pass | guaranteed | `wasmparser` validation runs post-emit in debug builds |

### T3 — `wasm32-wasi-p2` (GC-native, canonical target)

| Surface | Status | Detail |
|---------|--------|--------|
| parse | guaranteed | shared frontend; same parser as T1 |
| typecheck | guaranteed | shared frontend; same typechecker as T1 |
| compile (core Wasm) | guaranteed | 175 `t3-run` + 170 `t3-compile` fixtures |
| run (wasmtime) | guaranteed | 175 `t3-run` fixtures with stdout comparison (uses null GC collector) |
| emit component | smoke | 16 `component-compile` fixtures; skipped if `wasm-tools` absent |
| emit WIT | smoke | `--emit wit` tested in component-compile fixtures |
| host capabilities | guaranteed | WASI P2 imports conditionally emitted per reachability |
| determinism | smoke | spot-checked via baselines |
| validator pass | guaranteed | `wasmparser` validation runs post-emit |
| compile-error | guaranteed | 3 `compile-error` fixtures verify expected failures |

### T2 — `wasm32-freestanding` (Wasm GC, no WASI)

**Status: scaffold**

T2 is defined (ADR-007 / ADR-013) as a **Wasm GC, WASI-free** browser-oriented
target. The current implementation is still a **plumbing scaffold**: emitter
code lives in `crates/ark-wasm/src/emit/t2_freestanding.rs` and emits a minimal
**core Wasm** module (one page of linear memory, empty `_start`, **no** imports)
to prove the target and validation path before full MIR → Wasm GC lowering lands.

Repo-visible proof is `cargo test -p arukellt --test t2_scaffold`, which runs
`arukellt compile --target wasm32-freestanding` on the manifest fixture
`tests/fixtures/t2/t2_scaffold.ark` (also listed as `run:t2/t2_scaffold.ark` in
`tests/fixtures/manifest.txt`), then runs `wasmparser::Validator::validate_all`
and asserts no WASI imports and exports for `"memory"` and `"_start"`. An older
copy under `tests/fixtures/regression/t2_scaffold.ark` remains for inventory
compatibility but the canonical proof path is `tests/fixtures/t2/`.

The long-term I/O surface remains [ADR-020](adr/ADR-020-t2-io-surface.md); the
scaffold does not import `arukellt_io` yet.

T2 is a **v2 playground target** only in the roadmap sense.  Playground v1 does
not require T2 and is not blocked on it (see ADR-017).

| Surface | Status | Detail |
|---------|--------|--------|
| I/O surface | ADR written | ADR-020 defines the long-term `arukellt_io.write`/`flush` contract |
| compile (core Wasm) | scaffold | `cargo test -p arukellt --test t2_scaffold` compiles `--target wasm32-freestanding` against `tests/fixtures/t2/t2_scaffold.ark` and validates the emitted module |
| run | none | No runtime/browser execution support yet |
| validator pass | scaffold | Dedicated `t2_scaffold` proof runs `wasmparser::Validator::validate_all` on emitted output |

### T4 — native (LLVM backend)

**Status: scaffold**

The `crates/ark-llvm` crate exists and can emit basic LLVM IR for simple programs,
but it is excluded from the default build (`--exclude ark-llvm`) because it requires
LLVM 18.  No test infrastructure is wired up; correctness is unknown.  The crate is
kept for future native-target work but should not be advertised as functional.

| Surface | Status | Detail |
|---------|--------|--------|
| parse / typecheck | guaranteed | shared frontend |
| compile | scaffold | `crates/ark-llvm` exists, excluded from default build (requires LLVM 18) |
| run | none | no test infrastructure |
| emit | scaffold | basic LLVM IR emission exists |
| determinism | none | not applicable until compile is functional |

### T5 — interpreter / WASI P3

**Status: not-started**

T5 (interpreter or WASI P3 async) is not started.  The `wasm32-wasi-p3` target
identifier exists in `ark-target` but no codegen backend or runtime handles it.
There is no scaffold code.

| Surface | Status | Detail |
|---------|--------|--------|
| all | none | T5 is not implemented.  No code, no tests, no scaffold. |

## CI job mapping

| CI job | Target | What runs |
|--------|--------|-----------|
| `correctness` | all | `verify-harness.sh --cargo --size --wat --docs` |
| `target-behavior (wasm32-wasi-p1)` | T1 | `ARUKELLT_TARGET=wasm32-wasi-p1 cargo test -p arukellt --test harness` |
| `target-behavior (wasm32-wasi-p2)` | T3 | `ARUKELLT_TARGET=wasm32-wasi-p2 cargo test -p arukellt --test harness` |
| `perf-baseline` | T1 | `scripts/util/collect-baseline.py` (push-only) |

## Component output: separate guarantee tier

`--emit component` requires external `wasm-tools` and a WASI adapter
module.  These are not bundled with the Arukellt binary.  Therefore:

- Core Wasm output (`--emit core-wasm`) is **guaranteed** for T1 and T3.
- Component output (`--emit component`) is **smoke** tier for T3.
- If `wasm-tools` is not installed, component-compile fixtures are
  skipped, not failed.
- The CI environment does not currently install `wasm-tools`, so
  component-compile is effectively skip-on-CI.

## Updating this document

This document should only be updated when:

1. A new target is implemented or reaches a new verification tier.
2. CI jobs are added or restructured.
3. Fixture counts change significantly (regenerate via manifest counts).

Current fixture counts (from `tests/fixtures/manifest.txt`):

```text
run:              342
module-run:        25
diag:              29
module-diag:        8
t3-run:           175
t3-compile:       170
component-compile: 16
compile-error:      3
bench:              5
────────────────────
total:            773
```
