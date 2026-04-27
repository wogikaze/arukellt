---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 598
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Language Surface Uplift: Expression-Level Comprehensions
**Parent**: #588
**Depends on**: —
**Track**: selfhost-frontend / language-design
**Orchestration class**: design-ready

---

## Summary

Child issue for #588 Phase 4 — Expression-Level Comprehensions.

Add a lightweight collection-construction form that complements statement-style `for`.

Example direction:
```ark
let ys = [x * 2 for x in xs if x > 0]
```

This must desugar explicitly into existing iteration / builder constructs.
It is a syntax improvement, not a new evaluation strategy.

Initial restriction: array/Vec-style construction only. One generator, one optional filter.
Nested generators and local declarations may be deferred.

---

## Scope

**In scope:**
- Choose and freeze canonical comprehension syntax
- Support: one generator, one optional filter
- Element type inference and resulting collection type
- Explicit desugaring into existing iteration / builder machinery
- Positive and negative fixture coverage

**Out of scope:**
- Nested generators (`for x in xs for y in ys`) — defer after initial implementation
- Dictionary/map comprehensions — defer
- Lazy/generator comprehensions — explicitly out of scope (no new evaluation model)
- Any comprehension form that cannot be described as sugar over existing constructs

---

## Primary paths

- `src/compiler/parser.ark`
- `src/compiler/resolver.ark`
- `src/compiler/typechecker.ark`
- `src/compiler/corehir.ark`
- `tests/fixtures/selfhost/`

## Allowed adjacent paths

- `docs/language/syntax.md`
- `docs/language/spec.md`

---

## Upstream / Depends on

None directly (can proceed independently of #595–#597 in principle).
However, may be sequenced after #595–#597 for review bandwidth reasons.

## Blocks

- #599 (docs rollout covers all four features)

---

## Acceptance

1. Positive fixtures: map-only comprehension, map+filter comprehension
2. Negative fixtures: non-iterable source, filter expression not `bool`
3. Desugaring into existing constructs is explicit and deterministic
4. Existing `for` statement syntax is unchanged

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py selfhost fixture-parity
```

---

## STOP_IF

- Do not implement nested generators in the initial slice
- Do not add new collection runtime types to support comprehensions
- Do not change semantics of existing `for` statement

---

## Close gate

Close when the basic `[expr for x in iter if cond]` form parses, typechecks, and lowers
correctly, with positive and negative fixture coverage.