# Test Strategy

This document defines the test categories used in the Arukellt project,
their responsibilities, and how they map to the CI pipeline.

## Category overview

| Category | Scope | Gate level | Runner |
|----------|-------|-----------|--------|
| **unit** | Individual functions / modules in compiler crates | merge-blocking | `unit-tests` job: clippy, rustfmt, `cargo test --workspace --lib --bins` |
| **fixture** | End-to-end `.ark` → stdout/diagnostic correctness | mixed: T3 merge-blocking, T1 non-blocking | `fixture-primary` and `fixture-supported` jobs |
| **target-contract** | Per-target behavior and CI/doc target drift | mixed: T3 merge-blocking, T1 non-blocking, drift merge-blocking | `fixture-primary`, `fixture-supported`, and `target-contract-drift-check` |
| **component-interop** | Component Model emit + host interop | push-only informational | `component-interop` job: `bash scripts/manager.py --component` |
| **package-workspace** | `ark.toml`, workspace resolution, manifest, script execution | merge-blocking | `verification-package-workspace` job: `bash scripts/run/test-package-workspace.sh` |
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

## Failure Reporting

Local verification failures report the responsible category, command, and
primary path next to the failed check. Use those fields to decide which owner
or test lane should be investigated before opening the full log. The category
values match the table above; examples include `fixture`,
`component-interop`, `package-workspace`, `bootstrap`, `editor-tooling`,
`target-contract`, `perf`, and `docs`.

The metadata is emitted by `scripts/verify/harness.py` and by the full local
gate in `scripts/gate_domain/checks.py`. CI jobs still keep their own job names,
and the `CI category summary` job publishes the same vocabulary to the run job
summary plus the `ci-category-summary-<run_id>` artifact. Reviewers should open
that summary first when a run fails, then follow the responsible job link for
the failed category.

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
| **Unit tests** (`unit-tests`) | merge-blocking | unit | Also runs clippy and rustfmt so compiler regressions fail in the first layer. |
| **Package/workspace verification** (`verification-package-workspace`) | merge-blocking | package-workspace | Runs `bash scripts/run/test-package-workspace.sh`, covering manifest discovery and `ark.toml` script execution behavior. |
| **Verification harness — quick gate** (`verification-harness-quick`) | merge-blocking | docs/size/WAT auxiliary checks (quick slice) | Runs `python scripts/manager.py verify quick` in its own job so manifest / docs hygiene / repo-structure failures identify this layer immediately (distinct from `unit-tests`). |
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
| **CI category summary** (`ci-category-summary`) | reporting only | all named verification categories | Always runs and writes the category state table to the GitHub job summary and `ci-category-summary-<run_id>` artifact. |

Not every category has its own dedicated job yet. In particular,
`diagnostics-snapshot` still rides inside broader jobs, while
`component-interop` and `perf` remain push-only lanes. That is current
truth, and the names above are the ones to use when identifying which CI layer
failed.

The category summary records these piggyback mappings explicitly:
`package-workspace` maps to `verification-package-workspace`,
`diagnostics-snapshot` maps to `fixture-primary`, and the selfhost LSP lifecycle check maps to
`verification-harness-quick`. Push-only lanes appear as `skipped` on pull
requests, which is expected.

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
| package-workspace | dedicated `verification-package-workspace` job for manifest discovery and script execution | active |
| bootstrap | `selfhost-bootstrap` enforces Stage 0/1 and records Stage 2/parity evidence | partial |
| editor-tooling | 25 automated tests across `extension-tests` and `lsp-e2e` | active |
| determinism | dedicated `determinism` CI job | active |
| perf | 5 benchmark fixtures | active |
| diagnostics-snapshot | MIR + diagnostics snapshots | partial |
