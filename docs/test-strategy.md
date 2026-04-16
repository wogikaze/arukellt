# Test Strategy

This document defines the test categories used in the Arukellt project,
their responsibilities, and how they map to the CI pipeline.

## Category overview

| Category | Scope | Gate level | Runner |
|----------|-------|-----------|--------|
| **unit** | Individual functions / modules in compiler crates | merge-blocking | `unit-tests` job: clippy, rustfmt, `cargo test --workspace --lib --bins` |
| **fixture** | End-to-end `.ark` → stdout/diagnostic correctness | mixed: T3 merge-blocking, T1 non-blocking | `fixture-primary` and `fixture-supported` jobs |
| **target-contract** | Per-target behavior and CI/doc target drift | mixed: T3 merge-blocking, T1 non-blocking, drift merge-blocking | `fixture-primary`, `fixture-supported`, and `target-contract-drift-check` |
| **component-interop** | Component Model emit + host interop | push-only informational | `component-interop` job: `bash scripts/run/verify-harness.sh --component` |
| **package-workspace** | `ark.toml`, workspace resolution, manifest | merge-blocking, but currently piggybacks another layer | `unit-tests` job via `ark-manifest` / `ark-resolve` tests; no dedicated CI job yet |
| **bootstrap** | Selfhost Stage 0→1→2 bootstrap and parity evidence | mixed: Stage 0/1 merge-blocking, Stage 2/parity informational | `selfhost-bootstrap` job |
| **editor-tooling** | VS Code extension activation and LSP protocol behavior | merge-blocking | `extension-tests` and `lsp-e2e` jobs |
| **determinism** | Same input → same output | merge-blocking | `determinism` job |
| **perf** | Compile/run time regression | push-only informational | `perf-baseline` job |
| **diagnostics-snapshot** | Error message stability | merge-blocking when exercised by fixture diagnostics; no dedicated CI job | `fixture-primary` / `fixture-supported` for manifest-driven diagnostics |

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

The workflow in `.github/workflows/ci.yml` is the canonical CI layer list. The
current structure is already broader than the older `correctness` /
`target-behavior` wording, so this document tracks the jobs that actually exist
today.

| CI layer / job | Gate level | Primary categories covered | Notes |
|----------------|------------|----------------------------|-------|
| **Unit tests** (`unit-tests`) | merge-blocking | unit, package-workspace | Also runs clippy and rustfmt so compiler / manifest regressions fail in the first layer. |
| **Verification harness — quick gate** (`verification-harness-quick`) | merge-blocking | docs/size/WAT auxiliary checks (quick slice) | Runs `bash scripts/run/verify-harness.sh --quick` in its own job so manifest / docs hygiene / repo-structure failures identify this layer immediately (distinct from `unit-tests`). |
| **Fixture suite - T3 primary** (`fixture-primary`) | merge-blocking | fixture, target-contract | Primary target behavior gate for `wasm32-wasi-p2`. |
| **Fixture suite - T1 supported** (`fixture-supported`) | non-blocking | fixture, target-contract | Supported target alerting lane for `wasm32-wasi-p1`. |
| **Integration - CLI smoke** (`integration`) | merge-blocking | integration | Confirms release CLI can compile and run a known program. |
| **Packaging - binary smoke** (`packaging`) | merge-blocking | packaging CI layer | Verifies release binary entrypoints; this is a workflow layer rather than a top-level test category. |
| **Determinism - same input same output** (`determinism`) | merge-blocking | determinism | Byte-for-byte compile reproducibility gate. |
| **Heavy checks (size, WAT, docs)** (`heavy-checks`) | push-only | docs/size/WAT auxiliary checks | Executes `verify-harness.sh --size --wat --docs` (includes the same default harness checks as `--quick`, plus size/WAT/markdownlint); useful for drift detection, not a merge gate. |
| **Component interop** (`component-interop`) | push-only | component-interop | Optional component smoke coverage. |
| **Perf baseline snapshot** (`perf-baseline`) | push-only | perf | Collects baseline JSON artifacts. |
| **Selfhost bootstrap (full)** (`selfhost-bootstrap`) | merge-blocking with informational sub-results | bootstrap | Stage 0 and Stage 1 must pass; Stage 2 fixpoint and parity evidence are collected without failing the job. |
| **VS Code extension tests** (`extension-tests`) | merge-blocking | editor-tooling | Extension activation and feature workflow coverage. |
| **LSP E2E tests** (`lsp-e2e`) | merge-blocking | editor-tooling | Protocol-level LSP regression lane. |
| **Target contract drift check** (`target-contract-drift-check`) | merge-blocking | target-contract | Fails when `docs/target-contract.md` drifts from CI-described target truth. |
| **Final Gate** (`verify`) | merge-blocking aggregator | required merge gates | Summary gate over the required blocking layers. |

Not every category has its own dedicated job yet. In particular,
`package-workspace` and `diagnostics-snapshot` still ride inside broader jobs,
while `component-interop` and `perf` remain push-only lanes. That is current
truth, and the names above are the ones to use when identifying which CI layer
failed.

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
| target-contract | 247 (T1) + 182 (T3) via ARUKELLT_TARGET, plus drift enforcement in `target-contract-drift-check` | active |
| component-interop | 6 component-compile + 1 jco smoke | partial |
| package-workspace | ark-manifest / ark-resolve tests in `unit-tests` | partial |
| bootstrap | `selfhost-bootstrap` enforces Stage 0/1 and records Stage 2/parity evidence | partial |
| editor-tooling | 25 automated tests across `extension-tests` and `lsp-e2e` | active |
| determinism | dedicated `determinism` CI job | active |
| perf | 5 benchmark fixtures | active |
| diagnostics-snapshot | MIR + diagnostics snapshots | partial |
