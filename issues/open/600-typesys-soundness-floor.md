---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 600
Track: selfhost / typechecker
Orchestration class: design-ready
Depends on: —
Parent: None
Phase 0 baseline record is also part of this issue: run all verify/parity commands,
In scope: 
Out of scope: 
Note: "#312 (generic monomorphization) and #495 (trait bounds) are upstream blockers for"
Close when: occurs check is implemented, function boundary enforcement is active,
---

# Type System Stage-Up: Soundness Floor
- Phase 0: "Record baseline (observe only, no implementation)"
- Function boundary contracts: compare return expressions and final body type against declared return type
# Type System Stage-Up: Soundness Floor

---

## Summary

Child issue for #589 Phase 1 — Soundness Floor.

Before principled polymorphism can be layered onto the selfhost typechecker, the existing
inference engine must be made sound and explicit. This issue establishes the foundation:
occurs check, deep total substitution, function boundary contracts, and expanded exhaustiveness.

**Phase 0 baseline record is also part of this issue:** run all verify/parity commands,
record counts, write the gap ledger described in #589 Phase 0 before starting implementation.

---

## Scope

**In scope:**
- Phase 0: Record baseline (observe only, no implementation)
- `occurs_in_type` — reject bind_var when var occurs inside the target type
- Deep / total substitution for nested type arguments at all comparison and specialization boundaries
- Function boundary contracts: compare return expressions and final body type against declared return type
- Exhaust pattern coverage beyond bool-only toward current enums / Option / Result

**Out of scope:**
- TypeScheme / generalization — that is #601
- Obligation-based trait solving — that is #602
- Monomorphization contract — that is #603
- Any new surface syntax changes

---

## Primary paths

- `src/compiler/typechecker.ark`
- `tests/fixtures/selfhost/` (new negative fixtures for occurs check, function boundary)

## Allowed adjacent paths

- `docs/language/spec.md` (only to record the gap — no new spec surface this issue)

---

## Upstream / Depends on

None.
Note: #312 (generic monomorphization) and #495 (trait bounds) are upstream blockers for
later phases, but do not block this issue.

## Blocks

- #601 (TypeScheme work requires soundness floor to be stable)
- #312 (monomorphization gap partially closeable after deep substitution is fixed)

---

## Acceptance

1. `occurs_in_type` is implemented and `bind_var` rejects infinite/self-referential types
2. A new negative fixture `tests/fixtures/selfhost/infer_occurs_check.ark` is added and passes
3. Function bodies whose final expression type mismatches the declared return type produce a diagnostic
4. Selfhost exhaustiveness checker handles Option / Result patterns (not only bool)

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

Record baseline counts before any implementation.

---

## STOP_IF

- Do not start TypeScheme / generalization in this issue
- Do not start obligation-based solving in this issue
- Do not change surface syntax

---

## Close gate

Close when: occurs check is implemented, function boundary enforcement is active,
exhaustiveness covers Option/Result, all four selfhost parity gates have no new FAILs.