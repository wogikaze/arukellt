---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 602
Track: selfhost / typechecker
Orchestration class: blocked-by-upstream
Depends on: 601
Parent: #589
In scope: 
Out of scope: 
See also: "#495 (selfhost trait bounds open issue) and #312 (generic monomorphization) —"
Close when: obligation-based solving is active, coherence is enforced, ambiguity is
---

# Type System Stage-Up: Qualified Constraints and Coherent Trait Solving
discipline. Enforce coherence: "one visible impl per `(Trait, SelfType)` pair. Reject"
- Ambiguity checks: reject signatures and call sites that cannot be solved to a unique meaning
- Fixtures: "positive (trait bound satisfied), negative (unresolved, ambiguous, overlapping impl)"
- `where` trait-bound syntax changes (use current `T: Trait` style)
# Type System Stage-Up: Qualified Constraints and Coherent Trait Solving

---

## Summary

Child issue for #589 Phase 3 — Qualified Constraints + Coherent Trait Solving.

Upgrade selfhost traits from shallow checks into an obligation-based static overloading
discipline. Enforce coherence: one visible impl per `(Trait, SelfType)` pair. Reject
ambiguity. Make ADR-004's coherence requirements concrete and enforced.

---

## Scope

**In scope:**
- Replace "unresolved vars auto-pass trait bounds" with an obligation queue
- Obligations accumulate during inference and are discharged post-unification
- Unsolved obligations at close-out become errors, not silent success
- One impl per `(Trait, SelfType)` — overlapping impls rejected
- Ambiguity checks: reject signatures and call sites that cannot be solved to a unique meaning
- Fixtures: positive (trait bound satisfied), negative (unresolved, ambiguous, overlapping impl)

**Out of scope:**
- Associated types, default methods, specialization
- Dynamic dispatch (`dyn`) — explicitly out of scope
- Full orphan rules (implement the strongest locally enforceable subset)
- `where` trait-bound syntax changes (use current `T: Trait` style)

---

## Primary paths

- `src/compiler/typechecker.ark`
- `tests/fixtures/selfhost/` (trait bound positive + negative fixtures)
- `docs/adr/ADR-004-trait-strategy.md` (update with final coherence policy decision)

## Allowed adjacent paths

- `docs/language/spec.md` (coherence / orphan rule section)

---

## Upstream / Depends on

601 (TypeScheme — qualified constraints attach to schemes)
See also: #495 (selfhost trait bounds open issue) and #312 (generic monomorphization) —
those must align with the obligation model introduced here.

## Blocks

- #603 (lowering contract requires trait solving to be deterministic)
- #495 (obligation model closes the core of #495)

---

## Acceptance

1. `type_satisfies_trait_bound` no longer silently passes for unresolved type variables
2. A new negative fixture `tests/fixtures/selfhost/trait_unresolved_var_bound.ark` passes
3. Overlapping impl is rejected with a clear diagnostic
4. An ambiguous call site is rejected
5. ADR-004 is updated to reflect the final coherence decision

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

---

## STOP_IF

- Do not add `dyn` trait objects
- Do not implement associated types in this issue
- Do not implement specialization

---

## Close gate

Close when: obligation-based solving is active, coherence is enforced, ambiguity is
rejected, fixtures demonstrate all three cases, and ADR-004 is updated.