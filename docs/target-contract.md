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
| **scaffold** | Code exists but is not wired into any automated check. |
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

### T2 — `wasm32-freestanding` (Wasm GC, no WASI — browser/embedded)

**Status: ADR written, emitter not started**

T2 is not implemented.  No codegen backend, no tests, no scaffold.  The target
identifier `wasm32-freestanding` is registered in `ark-target` but nothing downstream
handles it.

The I/O surface design has been decided in [ADR-020](adr/ADR-020-t2-io-surface.md):
T2 modules import `{ arukellt_io: { write(ptr, len), flush() } }` from the JS host
and export their linear memory as `"memory"`.  A 1-page (64 KB) linear memory region
is retained for I/O string marshaling; all other storage uses Wasm GC.

T2 is a **v2 playground target** — playground v1 does not require T2 and is not
blocked on it (see ADR-017).

To start the T2 emitter: implement a new codegen backend in `crates/ark-wasm` that
emits Wasm GC instructions (no WASI), emits the two `arukellt_io` imports, exports
`"memory"`, and lowers `println` to `write`+`flush` call pairs.  Wire it into the
backend plan in `crates/ark-driver` and add fixture entries.

| Surface | Status | Detail |
|---------|--------|--------|
| I/O surface | ADR written | ADR-020 defines `arukellt_io.write`/`flush` import contract |
| codegen | none | T2 emitter not started.  No codegen, no tests, no scaffold. |
| all other | none | Blocked on codegen implementation. |

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
| `perf-baseline` | T1 | `scripts/collect-baseline.py` (push-only) |

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
run:              209
module-run:        18
diag:              17
module-diag:        3
t3-run:             5
t3-compile:       161
component-compile:  6
compile-error:     10
bench:              5
────────────────────
total:            434
```
