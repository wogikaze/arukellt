---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 611
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Optimization Uplift: T3-Safe Runtime Unlock
**Parent**: #591
**Depends on**: 609
**Track**: compiler / runtime-perf
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #591 Phase 3 — Runtime Uplift (T3-Safe First).

T3 currently keeps multiple O2/O3 passes gated and dead function elimination disabled
because the export/reachability contract is not yet safe enough. This issue defines the
root set contract and re-enables T3 passes one by one, only with regression fixtures.

Runtime improvements must be judged by `guest_ms` on guest-dominated workloads, not
wall-clock total or startup-dominated programs.

---

## Scope

**In scope:**
- Define the T3 export/reachability root contract explicitly:
  - entry roots, exported roots, host-reachable roots, internal-call roots
  - document the rule in compiler docs and code comments
- Re-enable T3 dead function elimination (after root contract is in place)
- Add regression fixture for host-reachable but not locally-called functions
- Unlock gated T3 passes one-by-one: each unlock requires a dedicated regression fixture
  and a written safety reason
  - Passes remaining unsafe must stay gated and generate a dedicated blocker issue

**Out of scope:**
- Parallel runtime execution
- Profile-guided optimization
- New optimization algorithms
- Binary size section compaction (that is #612)

---

## Primary paths

- `crates/ark-mir/src/passes/` (T3 gated passes)
- `crates/ark-mir/src/passes/README.md` (pass gating list)
- `crates/ark-target/` (reachability and export planning)
- `tests/fixtures/` (T3-specific regression fixtures)

## Allowed adjacent paths

- `docs/process/wasm-size-reduction.md` (T3 size impact after dead-elim)
- `docs/compiler/` (root contract documentation)

---

## Upstream / Depends on

609 (accurate T3 baselines needed to measure improvement)

## Blocks

- #612 (binary size work may overlap with dead-elim output)

---

## Acceptance

1. T3 reachability root contract is documented (entry / exported / host-reachable / internal)
2. T3 dead function elimination is re-enabled with at least one regression fixture
3. Each additionally unlocked gated pass has a written safety reason and a regression fixture
4. `guest_ms` improves on at least one guest-dominated benchmark (binary_tree or parse_tree_distance)

---

## Required verification

```bash
python scripts/manager.py perf benchmarks --no-quick
python scripts/manager.py perf gate
python scripts/manager.py verify --full
```

---

## STOP_IF

- Do not unlock passes without a regression fixture
- Do not use startup-dominated benchmarks to claim runtime wins
- Do not implement PGO

---

## Close gate

Close when: root contract is documented, dead-elim is re-enabled with fixture, at least
one other gated pass is unlocked with fixture and safety note, and `guest_ms` improves
on a guest-dominated benchmark.