# Test Strategy

This document defines the test categories used in the Arukellt project,
their responsibilities, and how they map to the CI pipeline.

## Category overview

| Category | Scope | Gate level | Runner |
|----------|-------|-----------|--------|
| **verification** | Selfhost compiler, manifest, docs, and policy checks | merge-blocking | `verification` + `docs` jobs |
| **fixture** | End-to-end `.ark` → stdout/diagnostic correctness | merge-blocking (via verification harness) | `verification` job (`manager.py verify` / fixtures) |
| **target-contract** | Per-target behavior and CI/doc target drift | merge-blocking when exercised in verification | `verification` job |
| **component-interop** | Component Model host interop fixture set | included in full verification | `python3 scripts/manager.py verify component-interop` |
| **component-emit** | Component/WIT library emit smoke | nonblocking release evidence | `python3 scripts/check/gate-666-component-library-emit.py` |
| **package-workspace** | `ark.toml`, workspace resolution, manifest, script execution | local / targeted | `bash scripts/run/test-package-workspace.sh` |
| **bootstrap** | Selfhost fixpoint and parity (ADR-029) | merge-blocking | `selfhost` job |
| **editor-tooling** | VS Code extension activation and LSP protocol behavior | merge-blocking | `extension-tests` job (+ LSP/DAP checks inside verification) |
| **determinism** | Same input → same output | exercised in verification / release checklist | no dedicated top-level job in `ci.yml` |
| **perf** | Compile/run time regression | local / targeted | `python3 scripts/util/collect-baseline.py` |
| **diagnostics-snapshot** | Error message stability | merge-blocking when exercised by fixtures / selfhost diag-parity | `verification` / `selfhost` |

## Regression layer mapping

| Layer | Categories that detect regressions |
|-------|-----------------------------------|
| **Language** (syntax, types, semantics) | verification, fixture, diagnostics-snapshot |
| **Backend** (codegen, optimization, emit) | fixture, target-contract, component-interop, determinism |
| **Tooling** (CLI, LSP, extension, DAP) | editor-tooling, package-workspace, bootstrap |

## Failure Reporting

Local verification failures report the responsible category, command, and
primary path next to the failed check. Category values match the table above.

## Fixture kinds and their categories

```text
run, module-run         → fixture (wasm32 / historical category)
diag, module-diag       → fixture (wasm32 / historical category)
t3-run                  → fixture (wasm32-gc; historical `t3-run:` prefix) / target-contract
t3-compile              → target-contract (wasm32-gc; historical `t3-*` prefix)
component-compile       → component-interop
compile-error           → target-contract (wasm32-gc; historical `t3-*` prefix)
bench                   → perf
```

## CI job structure

Canonical job IDs are generated from `.github/workflows/ci.yml`:

→ [`data/ci-jobs.md`](data/ci-jobs.md) (regenerate: `python3 scripts/gen/generate-ci-jobs-doc.py`)

Do **not** invent job names. Historical / incorrect names that must not appear
as current job IDs include:
`fixture-primary`, `fixture-supported`,
`target-contract-drift-check`, `verification-bootstrap`,
`verification-harness-quick`, `verification-package-workspace`,
`selfhost-bootstrap`, and a top-level `determinism` job.

## Adding a new test

1. Add a `.ark` fixture with `.expected` or `.diag` in `tests/fixtures/`.
2. Add the entry to `tests/fixtures/manifest.txt` with the correct kind prefix.
3. If the feature is primary (`wasm32-gc`)-specific, use historical fixture prefixes `t3-run:` or `t3-compile:` (internal category names, not public target IDs).
4. If it exercises component output, use `component-compile:` prefix.
5. Run `python3 scripts/manager.py verify fixtures` to verify.

## Current coverage

Counts below are **illustrative category labels**, not a second fixture SSOT.
Authoritative fixture totals: `docs/data/project-state.toml` → `[verification].fixture_manifest_count`
(and the harness pass/fail/skip fields).

| Category | Status |
|----------|--------|
| verification | active via `verification` / `docs` jobs |
| fixture | active via verification harness |
| target-contract | active via verification |
| component-interop | partial |
| package-workspace | local / script lane (not a top-level `ci.yml` job) |
| bootstrap | active via `selfhost` job (ADR-029) |
| editor-tooling | active via `extension-tests` |
| determinism | checklist / harness coverage; no dedicated job ID |
| perf | local / targeted |
| diagnostics-snapshot | partial via fixtures + selfhost diag-parity |
