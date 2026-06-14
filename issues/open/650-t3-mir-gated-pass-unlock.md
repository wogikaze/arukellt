---
Status: open
Created: 2026-06-15
Updated: 2026-06-15
ID: 650
Track: mir-opt
Depends on: 611
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Source: docs/current-state.md V4 — T3 dead-fn-elim disabled; T3_GATED_PASSES remain
---

# 650 — T3 MIR: unlock remaining gated O2/O3 passes and general dead-fn-elim

## Summary

Issue #611 landed T3-aware reachability pruning for **component/wit emit** paths.
`docs/current-state.md` still documents:

- Dead function elimination **disabled for T3** on the general wasm emit path
- Six O2/O3 passes gated until independently verified GC-safe (`T3_GATED_PASSES`)

Issues #080 / #082 / #083 (LICM, gc-hint, loop-unrolling) were reopened by false-done audit
and may not be ported to selfhost `src/compiler/passes/` yet. This issue tracks the
remaining T3 optimization unlock work after #611's component-scoped slice.

## Evidence

- `docs/current-state.md` § V4 Optimization Status
- `issues/done/611-opt-t3-unlock.md` — acceptance scoped to component emit + export roots
- `docs/process/docs-gap-inventory-2026-06-12.md` — T3 dead-fn-elim listed as policy-by-design
  (revisit now that #611 partial land exists)

## Non-goals

- PGO / profile-guided optimization
- Startup-dominated benchmark claims

## Acceptance

- [ ] `docs/compiler/t3-reachability.md` (or successor) covers general wasm emit roots, not only component
- [ ] T3 dead function elimination re-enabled for `--emit wasm` with regression fixture
- [ ] Each newly unlocked gated O2/O3 pass has a written GC-safety note + fixture
- [ ] `docs/current-state.md` V4 section updated (remove stale blanket "disabled for T3" if fixed)
- [ ] Guest-dominated benchmark shows non-regression or improvement on at least one workload
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Required verification

```bash
python3 scripts/manager.py verify quick
python3 scripts/manager.py perf benchmarks --no-quick  # when perf claims made
```

## Close gate

General T3 wasm emit uses the documented reachability contract; at least one previously
gated pass is unlocked with fixture evidence; current-state matches code.
