---
Status: done
Created: 2026-05-16
Updated: 2026-05-17
ID: 627
Track: main
Parent: 529
Orchestration class: done
Depends on: 626
Blocks: 628
---

# 529 Phase 6/B: Analysis API

## Summary

Phase 6/B of #529: extract a reusable analysis API from the CLI subprocess model. The new entry point provides `document text -> AST / symbols / diagnostics` without requiring CLI invocation.

This enables LSP and other IDE tooling to use the selfhost compiler as a library rather than a subprocess.

## Acceptance

- [x] Analysis API exists with a library entry point (`src/compiler/analysis.ark`; actual path chosen in #568).
- [x] Entry point provides source text -> AST-derived counts, symbol data, and diagnostics.
- [x] Entry point does NOT require CLI argument parsing — callable programmatically.
- [x] No `arukellt compile` / `arukellt check` subprocess invocation required for analysis.
- [x] API is usable from LSP (`src/compiler/lsp.ark`) without shelling out.
- [x] No SKIP added to `scripts/selfhost/checks.py`.
- [x] 4 canonical selfhost gates green with FAIL=0 and SKIP delta = 0.
- [x] Runner test drives the API directly and asserts expected structure (`scripts/check/check-analysis-api.py`).

## Scope

**In scope:**
- Library-style entry point in `src/ide/api.ark`
- Source text -> AST, symbols, diagnostics pipeline
- Error recovery awareness (leverages Phase 6/A)
- Test coverage for API shape

**Out of scope:**
- LSP protocol handlers (Phase 6/C)
- Incremental update support (requires #099)
- Performance optimization for sub-millisecond response times
- DAP support (Phase 6/D, tracked by #571)

## Primary paths

- `src/ide/api.ark` (new file — analysis API)
- `src/compiler/driver.ark` (may need refactoring to expose programmatic entry)

## Allowed adjacent paths

- `tests/fixtures/ide/` (new test fixtures)
- `tests/fixtures/manifest.toml`

## Upstream / Depends on

- #626 (Phase 6/A: IDE-Ready Frontend) — requires error recovery in frontend components before the API is useful

## Blocks

- #628 (Phase 6/C: LSP Minimum Viable) — LSP handlers use the analysis API

## Required verification (close gate)

```bash
python scripts/manager.py verify
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

## STOP_IF

- Any of the 4 canonical selfhost gates regresses (FAIL>0 or SKIP delta > 0) — revert and STOP
- The API design requires changes to forbidden paths (other crates/) — open a sibling issue and STOP
- Batch compiler pipeline is broken by API refactoring — revert and STOP
- Scope expands to include LSP protocol handling — open sibling issue and STOP

## Close gate

Close when the analysis API exists as a programmatic entry point, required verification passes with FAIL=0 and SKIP delta = 0, and at least one test validates the API end-to-end.

## Close Note (2026-05-17)

Closed as the aggregate parent for #568. The original `src/ide/api.ark` path was
resolved in #568 to `src/compiler/analysis.ark` so the selfhost module loader can
import it beside `main.ark`.

Evidence:

- `src/compiler/analysis.ark` exports `analysis::analyze(uri, text)`.
- `src/compiler/lsp.ark` consumes `analysis::analyze` directly.
- `python3 scripts/check/check-analysis-api.py`: PASS, 3 fixtures.
- Current `python scripts/manager.py verify quick`: PASS, 23/23.


## Audit resolution — 2026-06-12 (Slice E)

**Classification:** `truly-done`

**Repo proof:** Acceptance satisfied on selfhost/extension path after cross-check of lifecycle scripts (`scripts/check/check-lsp-lifecycle.py`, `check-dap-lifecycle.py`, `check-analysis-api.py`), `tests/fixtures/selfhost/`, and `extensions/arukellt-all-in-one/`.

**Action:** Kept in `issues/done/`. Prior `Reopened by audit` banners (2026-04-03) were orchestration drift, not current product false-done.
