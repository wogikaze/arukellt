---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 603
Track: selfhost / typechecker / lowering
Orchestration class: blocked-by-upstream
Depends on: 601, 602
Parent: None
In scope: 
Out of scope: 
Close when: end-to-end generic lowering is deterministic, no unresolved trait dispatch
---

end-to-end lowering contract: generalized + constrained source types lower into
# Type System Stage-Up: Monomorphization and Lowering Contract Closure
- Close #312: finish generic specialization for direct calls, method calls, nested generics
- Typechecker → CoreHIR/MIR contract: carry enough information to materialize concrete
- Parity guardrails: ensure selfhost typechecker output is semantically equivalent to
# Type System Stage-Up: Monomorphization and Lowering Contract Closure

---

## Summary

Child issue for #589 Phase 4 — Monomorphization / Lowering Contract Closure.

After TypeScheme (#601) and obligation-based trait solving (#602), this issue closes the
end-to-end lowering contract: generalized + constrained source types lower into
deterministic monomorphic CoreHIR/MIR. This also closes #312 (selfhost generic
monomorphization) as a dependency.

---

## Scope

**In scope:**
- Close #312: finish generic specialization for direct calls, method calls, nested generics
- Typechecker → CoreHIR/MIR contract: carry enough information to materialize concrete
  instances without ad hoc fallback
- Trait method resolution must produce a deterministic concrete callee before or during lowering
- No unresolved trait dispatch entering the backend
- Parity guardrails: ensure selfhost typechecker output is semantically equivalent to
  the Rust typechecker output for the closure cases
- Close note for #312 if the monomorphization gap is fully resolved

**Out of scope:**
- Rust typechecker crate deletion (that is #577 — only after selfhost semantic parity is proven)
- New surface syntax
- Associated types, specialization

---

## Primary paths

- `src/compiler/typechecker.ark`
- `src/compiler/corehir.ark`
- `src/compiler/mir.ark`
- `tests/fixtures/selfhost/` (monomorphization fixtures)

## Allowed adjacent paths

- `issues/open/312-selfhost-generic-monomorphization.md` (close note candidate)
- `crates/ark-typecheck/` (read-only reference, do not modify)

---

## Upstream / Depends on

601, 602

## Blocks

- #577 (Rust typechecker deletion — only after selfhost lowering is proven complete)
- #312 (resolved by this issue)

---

## Acceptance

1. Generic functions specializing over nested type arguments lower without ad hoc fallback
2. Trait method calls resolve to a concrete callee before backend handoff
3. A new multi-param generic fixture passes in the selfhost path
4. All four selfhost parity gates show no new FAILs
5. #312 can be closed with a close note referencing this issue

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
python scripts/manager.py selfhost parity --mode --cli
```

---

## STOP_IF

- Do not delete `crates/ark-typecheck` in this issue — that is #577
- Do not implement specialization
- Do not add new surface types

---

## Close gate

Close when: end-to-end generic lowering is deterministic, no unresolved trait dispatch
reaches the backend, selfhost parity gates are clean, and #312 has a close note.