---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 601
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Type System Stage-Up: Type Schemes and Controlled Let-Generalization
**Parent**: #589
**Depends on**: 600
**Track**: selfhost / typechecker
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #589 Phase 2 — Type Schemes + Controlled Let-Generalization.

Introduce an internal `TypeScheme` representation and conservative let-generalization policy.
The goal is for local polymorphic helpers to work in selfhost fixtures, with repeated use
instantiating fresh variables, and with the generalization rule explicitly documented.

**Policy constraint:** Do NOT generalize everything blindly. Use a conservative
syntactic value / non-expansive rule or equivalent. Document the rule in spec + fixtures.

---

## Scope

**In scope:**
- Internal `TypeScheme` type: quantified variables + deferred trait constraints + body type
- `scope_lookup` instantiates fresh type variables from stored schemes on each use
- `generalize` at eligible top-level items
- Conservative local `let` generalization (non-expansive rule or documented equivalent)
- Write the exact generalization rule in `docs/language/spec.md` + fixtures

**Out of scope:**
- Obligation-based trait solving (that is #602)
- Monomorphization contract closure (that is #603)
- Associated types, default methods, specialization
- Changes to the public surface syntax for generic items

---

## Primary paths

- `src/compiler/typechecker.ark`
- `tests/fixtures/selfhost/` (polymorphic local helper fixtures)
- `docs/language/spec.md` (generalization rule section)

## Allowed adjacent paths

- `src/compiler/corehir.ark` (only if TypeScheme needs a lowering hook)

---

## Upstream / Depends on

600 (soundness floor must be established — no principled polymorphism on unsound substitution)

## Blocks

- #602 (obligation-based solving builds on TypeScheme)
- #603 (lowering contract assumes TypeScheme is stable)

---

## Acceptance

1. A polymorphic helper function can be used multiple times in a scope without aliasing the
   same type variable state (each use instantiates fresh vars)
2. A negative fixture rejects non-expansive generalization violations
3. The generalization rule is documented in spec and fixture expectations
4. All four selfhost parity gates show no new FAILs

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

---

## STOP_IF

- Do not implement "infer everything everywhere" generalization
- Do not start obligation-based solving
- Do not change the public surface type annotation syntax

---

## Close gate

Close when local polymorphic helpers work correctly in selfhost fixtures, fresh
instantiation is confirmed by fixtures, and the generalization rule is documented.