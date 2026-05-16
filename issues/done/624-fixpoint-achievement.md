---
Status: done
Created: 2026-05-16
Updated: 2026-05-16
ID: 624
Track: main
Parent: 529
Orchestration class: implementation-ready
Depends on: none
Blocks: 594
---

# 529 Phase 1: Fixpoint Achievement (CRITICAL)

## Summary

Phase 1 of #529: achieve `sha256(s2) == sha256(s3)` — stable, reproducible self-compilation of the selfhost compiler. This phase is the prerequisite for all subsequent phases. **Already achieved** per `docs/current-state.md`.

### What was done

The selfhost compiler (`src/compiler/*.ark`) now:
- Loads multi-file source modules recursively (`use` resolution, visited set, sorted order)
- Handles qualified cross-module calls in MIR/emitter
- Enforces determinism: no HashMap iteration in output-affecting paths, no filesystem order dependence, deterministic function index allocation

## Acceptance (all pre-verified)

- [x] `python scripts/manager.py selfhost fixpoint` passes (rc=0)
- [x] `bash scripts/run/verify-bootstrap.sh --check` reports `stage1-self-compile: reached` AND `stage2-fixpoint: reached` AND `attainment: reached`
- [x] `sha256(s2) == sha256(s3)` — fixpoint verified in CI
- [x] Determinism enforced: no HashMap iteration in output-affecting paths
- [x] Multi-file module loading implemented in `src/compiler/driver.ark`
- [x] Qualified call resolution aligned in `src/compiler/mir.ark` and `src/compiler/emitter.ark`

## Scope

**In scope:**
- Module loading specification (minimal: `use foo` resolves to `foo.ark` in same directory, recursive, deduplicated, sorted)
- Multi-file source loading in `src/compiler/driver.ark`
- Qualified cross-module call handling in `src/compiler/mir.ark` and `src/compiler/emitter.ark`
- Determinism enforcement (no nondeterministic HashMap, fs order, or function index allocation)

**NOT in scope (deferred to later phases):**
- Package system, complex search paths, import cycle recovery

## Primary paths

- `src/compiler/driver.ark` (module loading)
- `src/compiler/mir.ark` (qualified call resolution)
- `src/compiler/emitter.ark` (function index determinism)

## Upstream / Depends on

None (this is the first phase of #529).

## Blocks

- #594 (Phase 2: Fixture and Diagnostic Parity) — requires stable fixpoint before parity work
- Phase 4 (Dual-Run Period) — requires fixpoint as baseline

## Required verification

```bash
python scripts/manager.py selfhost fixpoint
bash scripts/run/verify-bootstrap.sh --check
```

## Phase 1 Exit Condition

`stage1-self-compile: reached` AND `stage2-fixpoint: reached`

**Current status (from docs/current-state.md):** ACHIEVED — fixpoint `attainment: reached`, `sha256(s2) == sha256(s3)` passes.

## STOP_IF

- Determinism regression: if fixpoint breaks after changes, stop and root-cause before any other work
- Do not proceed to Phase 2+ until Phase 1 fixpoint is stable (already verified)
