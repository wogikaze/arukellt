# Test Strategy

This document defines the test categories used in the Arukellt project,
their responsibilities, and how they map to the CI pipeline.

## Category overview

| Category | Scope | Gate level | Runner |
|----------|-------|-----------|--------|
| **unit** | Individual functions / modules in compiler crates | merge-blocking | `cargo test --workspace --lib` |
| **fixture** | End-to-end `.ark` → stdout/diagnostic correctness | merge-blocking | `cargo test -p arukellt --test harness` |
| **target-contract** | Per-target fixture subset via `ARUKELLT_TARGET` | merge-blocking | `target-behavior` CI matrix |
| **component-interop** | Component Model emit + host interop | smoke (opt-in) | `verify-harness.sh --component` |
| **package-workspace** | `ark.toml`, workspace resolution, manifest | merge-blocking | unit tests in `ark-manifest` / `ark-resolve` |
| **bootstrap** | Selfhost Stage 0→1→2 fixpoint | informational | `scripts/verify-bootstrap.sh` |
| **editor-tooling** | VS Code extension activate / LSP handshake | smoke (planned) | `@vscode/test-cli` (not yet wired) |
| **determinism** | Same input → same output | smoke | `tests/baselines/` comparison |
| **perf** | Compile/run time regression | non-blocking | `scripts/benchmark_runner.py` |
| **diagnostics-snapshot** | Error message stability | informational | `tests/snapshots/diagnostics/` |

## Regression layer mapping

When a test fails, the category tells you which subsystem to investigate:

| Layer | Categories that detect regressions |
|-------|-----------------------------------|
| **Language** (syntax, types, semantics) | unit, fixture, diagnostics-snapshot |
| **Backend** (codegen, optimization, emit) | fixture, target-contract, component-interop, determinism |
| **Tooling** (CLI, LSP, extension, DAP) | editor-tooling, package-workspace, bootstrap |

## Fixture kinds and their categories

```text
run, module-run         → fixture (T1)
diag, module-diag       → fixture (T1)
t3-run                  → fixture (T3) / target-contract
t3-compile              → target-contract (T3)
component-compile       → component-interop
compile-error           → target-contract (T3)
bench                   → perf
```

## CI job structure

### Merge-blocking jobs

1. **correctness** — `verify-harness.sh --cargo --size --wat --docs`
   - Runs: cargo fmt, clippy, workspace unit tests, size gate, WAT roundtrip, markdownlint
2. **target-behavior (wasm32-wasi-p1)** — T1 fixture subset
3. **target-behavior (wasm32-wasi-p2)** — T3 fixture subset

### Non-blocking / optional jobs

1. **perf-baseline** — push-only baseline collection
2. *(planned)* **editor-smoke** — VS Code extension tests
3. *(planned)* **bootstrap-check** — selfhost fixpoint verification
4. *(planned)* **determinism-check** — binary reproducibility

## Adding a new test

When adding a feature:

1. Add a `.ark` fixture with `.expected` or `.diag` in `tests/fixtures/`.
2. Add the entry to `tests/fixtures/manifest.txt` with the correct kind prefix.
3. If the feature is T3-specific, use `t3-run:` or `t3-compile:` prefix.
4. If it exercises component output, use `component-compile:` prefix.
5. Run `cargo test -p arukellt --test harness` to verify.

## Current coverage

| Category | Count | Status |
|----------|-------|--------|
| unit | ~150 tests across workspace crates | active |
| fixture | 434 manifest entries | active |
| target-contract | 247 (T1) + 182 (T3) via ARUKELLT_TARGET | active |
| component-interop | 6 component-compile + 1 jco smoke | partial |
| package-workspace | ark-manifest unit tests | partial |
| bootstrap | Stage 0 compiles, Stage 1/2 conditional | scaffold |
| editor-tooling | 0 automated tests | not started |
| determinism | baseline JSON comparison | partial |
| perf | 5 benchmark fixtures | active |
| diagnostics-snapshot | MIR + diagnostics snapshots | partial |
