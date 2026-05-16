---
Status: open
Created: 2026-04-22
Updated: 2026-05-16
ID: 611
Track: compiler / runtime-perf
Orchestration class: blocked-by-upstream
Depends on: 609 (DONE)
Dependency status: 609 is complete. Issue is TECHNICALLY UNBLOCKED but all primary paths are stale due to Rust crate retirement.
Assessment date: 2026-05-16
---

# Optimization Uplift: T3-Safe Runtime Unlock

---

## Assessment (2026-05-16)

### Blocking check: 609 DONE -- NOT truly blocked

Issue #609 (Measurement Truth Repair) was completed on 2026-04-23. The measurement
baselines and docs are in place. However, this issue has a **stale-path problem much
more severe than #610** because the entire Rust MIR crate has been removed since the
issue was written.

### Paths are all stale

The issue's "Primary paths" references these locations that no longer exist:

- **`crates/ark-mir/src/passes/`** -- The entire `crates/ark-mir` Rust crate was removed
  in #561. All MIR passes now live in the selfhost compiler (`src/compiler/`).
- **`crates/ark-mir/src/passes/README.md`** -- Likewise removed. There is no
  `src/compiler/passes/README.md` either (current-state.md references a selfhost
  `src/compiler/passes/` but the directory does not exist on the current branch).
- **T3_GATED_PASSES in `session.rs`** -- The Rust `ark-mir` crate (which contained
  `session.rs`) is gone. The gating mechanism is entirely in selfhost code now.

The "Allowed adjacent paths" references `crates/ark-target/` which has also evolved
during the selfhost migration.

### Current T3 state (verified 2026-05-16)

- T3 (wasm32-wasi-p2) is stable with full GC-native data model.
- T3 runs all 9 O1 MIR passes via selfhost pass infrastructure.
- Three O2 arithmetic passes are also active for T3 at O2: `algebraic_simplify`,
  `strength_reduction`, `string_concat_opt`.
- **Dead function elimination is still disabled for T3** -- the WASI export contract
  concern that motivated #611 remains unresolved.
- Six O2/O3 passes remain gated.
- The target metric (`guest_ms` on guest-dominated workloads) is measurable:
  binary_tree guest=9.771ms, parse_tree_distance guest=35.278ms in the current baselines.

### Recommendation

This issue is **not truly blocked by 609**, but it cannot be started as-is. It needs a
**full path audit and scope refresh** to reflect that:

1. All MIR/pass work lives in selfhost `src/compiler/*.ark` now.
2. T3 gating mechanisms are in the selfhost compiler, not in Rust crates.
3. The pass README/gating-list needs to be created (or the issue scope adjusted to create it).
4. Dead function elimination contract documentation would go in `docs/compiler/` per current conventions.

Despite the path staleness, the underlying goal is still relevant: dead function elimination
is disabled for T3, and six O2/O3 passes remain unsafe. This represents a genuine runtime
optimization opportunity.

---

## Summary

Child issue for #591 Phase 3 -- Runtime Uplift (T3-Safe First).

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

- **NOTE: All paths below need review. Original paths (`crates/ark-mir/`) were removed in #561.**
- `src/compiler/` (selfhost pass infrastructure -- current location of MIR passes)
- `src/compiler/mir_ir.ark`, `src/compiler/mir_lower.ark` (MIR IR and lowering)
- `docs/compiler/` (root contract documentation would go here)
- `tests/fixtures/` (T3-specific regression fixtures)

## Allowed adjacent paths

- `docs/process/wasm-size-reduction.md` (T3 size impact after dead-elim)
- `docs/compiler/` (root contract documentation)

---

## Upstream / Depends on

609 (accurate T3 baselines needed to measure improvement) -- DONE
**Stale-path alert:** All `crates/ark-mir/` references in this issue were invalidated by #561 (selfhost migration). Must be updated before implementation.

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
