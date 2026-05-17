# Test Strategy

This document defines the test categories used in the Arukellt project,
their responsibilities, and how they map to the CI pipeline.

## Category overview

| Category | Scope | Gate level | Runner |
|----------|-------|-----------|--------|
| **verification** | Selfhost compiler, manifest, docs, and policy checks | merge-blocking | `verification` job: `python3 scripts/manager.py verify` |
| **fixture** | End-to-end `.ark` → stdout/diagnostic correctness | mixed: T3 merge-blocking, T1 non-blocking | `fixture-primary` and `fixture-supported` jobs |
| **target-contract** | Per-target behavior and CI/doc target drift | mixed: T3 merge-blocking, T1 non-blocking, drift merge-blocking | `fixture-primary`, `fixture-supported`, and `target-contract-drift-check` |
| **component-interop** | Component Model emit + host interop | included in verification when available | `python3 scripts/manager.py verify --component` |
| **package-workspace** | `ark.toml`, workspace resolution, manifest, script execution | local / targeted | `bash scripts/run/test-package-workspace.sh` |
| **bootstrap** | Selfhost fixpoint and parity evidence | merge-blocking | `selfhost` job |
| **editor-tooling** | VS Code extension activation and LSP protocol behavior | merge-blocking | `extension-tests` job |
| **determinism** | Same input → same output | merge-blocking | `determinism` job |
| **perf** | Compile/run time regression | local / targeted | `python3 scripts/util/collect-baseline.py` |
| **diagnostics-snapshot** | Error message stability | merge-blocking when exercised by fixture diagnostics; no dedicated CI job | `fixture-primary` / `fixture-supported` for manifest-driven diagnostics |

## Regression layer mapping

When a test fails, the category tells you which subsystem to investigate:

| Layer | Categories that detect regressions |
|-------|-----------------------------------|
| **Language** (syntax, types, semantics) | verification, fixture, diagnostics-snapshot |
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
| **Package/workspace verification** (`verification-package-workspace`) | non-blocking alert | package-workspace | Runs `bash scripts/run/test-package-workspace.sh`, covering manifest discovery and `ark.toml` script execution behavior. This remains an alert lane until selfhost `build` / `script` semantics are implemented. |
| **Verification harness** (`verification`) | merge-blocking | verification, fixture, docs, target-contract | Runs `python3 scripts/manager.py verify`. |
| **Selfhost gates** (`selfhost`) | merge-blocking | bootstrap, fixture, diagnostics-snapshot | Runs fixpoint, fixture parity, CLI parity, and diagnostic parity. |
| **Docs consistency** (`docs`) | merge-blocking | docs | Runs `python3 scripts/check/check-docs-consistency.py`. |
| **Release tag version** (`release-tag`) | tag-only | packaging | Validates release tags against `docs/data/project-state.toml`. |
| **VS Code extension tests** (`extension-tests`) | merge-blocking | editor-tooling | Extension activation and feature workflow coverage against a live E2E CLI fixture. Selfhost LSP/DAP protocol lifecycle coverage remains in the verification harness. |
| **Final Gate** (`verify`) | merge-blocking aggregator | required merge gates | Summary gate over the required blocking layers. |

Not every category has its own dedicated job yet. In particular,
`diagnostics-snapshot` still rides inside broader jobs, while
`component-interop` and `perf` remain push-only lanes. That is current
truth, and the names above are the ones to use when identifying which CI layer
failed.

The category summary records these piggyback mappings explicitly:
`package-workspace` maps to the non-blocking `verification-package-workspace` alert lane,
`diagnostics-snapshot` maps to `fixture-primary`, and the selfhost LSP/DAP lifecycle checks map to
`verification-harness-quick`. Push-only lanes appear as `skipped` on pull
requests, which is expected.

## Adding a new test

When adding a feature:

1. Add a `.ark` fixture with `.expected` or `.diag` in `tests/fixtures/`.
2. Add the entry to `tests/fixtures/manifest.txt` with the correct kind prefix.
3. If the feature is T3-specific, use `t3-run:` or `t3-compile:` prefix.
4. If it exercises component output, use `component-compile:` prefix.
5. Run `python scripts/manager.py verify fixtures` to verify.

## Current coverage

| Category | Count | Status |
|----------|-------|--------|
| verification | selfhost and policy checks through `scripts/manager.py` | active |
| fixture | 434 manifest entries | active |
| target-contract | 247 (T1) + 182 (T3) via ARUKELLT_TARGET, plus drift enforcement in `target-contract-drift-check` | active |
| component-interop | 6 component-compile + 1 jco smoke | partial |
| package-workspace | dedicated non-blocking `verification-package-workspace` alert lane for manifest discovery and script execution | partial |
| bootstrap | `selfhost-bootstrap` enforces selfhost fixpoint; dedicated CLI/diagnostic parity jobs enforce parity | active |
| editor-tooling | automated VS Code extension tests in `extension-tests`; selfhost protocol lifecycle tests in `verification-harness-quick` | active |
| determinism | dedicated `determinism` CI job | active |
| perf | 5 benchmark fixtures | active |
| diagnostics-snapshot | MIR + diagnostics snapshots | partial |
