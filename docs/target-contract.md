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

## Target × Verification Surface

### T1 — `wasm32-wasi-p1` (CLI default)

| Surface | Status | Detail |
|---------|--------|--------|
| parse | guaranteed | 209 `run` + 18 `module-run` + 17 `diag` + 3 `module-diag` fixtures |
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
| compile (core Wasm) | guaranteed | 157 `t3-run` + 161 `t3-compile` fixtures |
| run (wasmtime) | guaranteed | 157 `t3-run` fixtures with stdout comparison (uses null GC collector) |
| emit component | smoke | 6 `component-compile` fixtures; skipped if `wasm-tools` absent |
| emit WIT | smoke | `--emit wit` tested in component-compile fixtures |
| host capabilities | guaranteed | WASI P2 imports conditionally emitted per reachability |
| determinism | smoke | spot-checked via baselines |
| validator pass | guaranteed | `wasmparser` validation runs post-emit |
| compile-error | guaranteed | 10 `compile-error` fixtures verify expected failures |

### T2 — `wasm32-freestanding` (Wasm GC, no WASI)

**Status: scaffold**

T2 now has a minimal emitter scaffold in `crates/ark-wasm`. It produces a
structurally valid core Wasm module for the freestanding target, exports
`"memory"` and `"_start"`, and passes `wasmparser` validation through the
dedicated proof in `cargo test -p arukellt --test t2_scaffold -- --nocapture`.
That test drives the CLI with `--target wasm32-freestanding` against
`tests/fixtures/regression/t2_scaffold.ark`. It does **not** implement real
MIR lowering yet and does not imply browser/runtime execution support.

The I/O surface design is still the long-term contract from [ADR-020](adr/ADR-020-t2-io-surface.md),
but the current scaffold does not wire that host bridge yet.

T2 is a **v2 playground target** only in the roadmap sense.  Playground v1 does
not require T2 and is not blocked on it (see ADR-017).

| Surface | Status | Detail |
|---------|--------|--------|
| I/O surface | ADR written | ADR-020 defines the long-term `arukellt_io.write`/`flush` contract |
| compile (core Wasm) | scaffold | `cargo test -p arukellt --test t2_scaffold -- --nocapture` compiles `--target wasm32-freestanding` and validates the emitted module |
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

Current fixture counts (as of this commit):

```text
run:              210
module-run:        18
diag:              17
module-diag:        3
t3-run:             5
t3-compile:       161
component-compile:  6
compile-error:     10
bench:              5
────────────────────
total:            435
```
