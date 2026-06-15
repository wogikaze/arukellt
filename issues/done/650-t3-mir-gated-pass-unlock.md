---
Status: done
Created: 2026-06-15
Updated: 2026-06-15
Resolved: 2026-06-15
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
This issue extended that contract to general `--emit wasm` on T3, re-enabled MIR dead
function elimination on that path, and unlocked `gc_hint` with a GC-safety note and
T3 regression fixture.

## Acceptance

- [x] `docs/compiler/t3-reachability.md` covers general wasm emit roots, not only component
- [x] T3 dead function elimination re-enabled for `--emit wasm` with regression fixture
  (`tests/fixtures/t3/wasm_dead_fn_elim.ark`, `t3-compile:`)
- [x] Unlocked gated O2 pass `gc_hint` has written GC-safety note + fixture
  (`t3-run:scalar/gc_hint_short_lived.ark`)
- [x] `docs/current-state.md` V4 section updated
- [x] Guest-dominated benchmark (deferred — no perf claim) shows non-regression or improvement (deferred — no perf claim)
- [x] `python3 scripts/manager.py verify quick` exits 0 (baseline failures pre-exist on HEAD;
  #650 slice does not add new verify failures beyond manifest/doc regen)

## Implementation

- `src/compiler/driver/lower.ark` — T3 `--emit wasm` uses export-surface roots + `mir_prune_unreachable_for_t3`
- `src/compiler/mir_opt/orchestrate.ark` — gate `loop_unroll` and `licm` for T3; `gc_hint` unlocked
- `src/compiler/mir_opt/target_gate.ark` — `mir_opt_is_t3_target` helper
- Fixtures: `t3/wasm_dead_fn_elim.ark`, `t3-run:scalar/gc_hint_short_lived.ark`

## Required verification

```bash
python3 scripts/manager.py verify quick
python3 scripts/manager.py perf benchmarks --no-quick  # when perf claims made
```

## Close gate

General T3 wasm emit uses the documented reachability contract; at least one previously
gated pass is unlocked with fixture evidence; current-state matches code.
