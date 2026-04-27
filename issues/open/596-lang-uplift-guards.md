---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 596
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Language Surface Uplift: Function-Level Guards
**Parent**: #588
**Depends on**: 595
**Track**: selfhost-frontend / language-design
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #588 Phase 2 — Function-Level Guards.

Reuse the existing guard expression model (already supported in `match` arms) and allow
guarded clauses on function definitions defined via the multi-clause syntax from #595.

Example direction:
```ark
fn classify(n) -> String | n > 0 = "pos"
fn classify(n) -> String | n < 0 = "neg"
fn classify(_) -> String = "zero"
```

---

## Scope

**In scope:**
- Guard syntax on function clause heads (reusing `match` guard expression model)
- Evaluation order: match pattern → bind → evaluate guard → choose body
- Positive and negative fixtures for guarded clauses

**Out of scope:**
- `where` clauses (that is #597)
- Comprehensions (that is #598)
- Guard-style matching in `match` arms (already exists, do not duplicate)
- New guard expression types beyond what `match` already supports

---

## Primary paths

- `src/compiler/parser.ark`
- `src/compiler/typechecker.ark`
- `src/compiler/corehir.ark`
- `tests/fixtures/selfhost/`

---

## Upstream / Depends on

595 (multi-clause fn syntax must exist before guards can attach to clause heads)

## Blocks

- #599 (docs rollout)

---

## Acceptance

1. Positive fixtures: literal + guard, enum + guard, struct destructure + guard
2. Negative fixtures: guard name not in scope, guard type is not `bool`
3. Guards evaluate after pattern match bindings are established
4. Clause ordering is stable and source-visible

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py selfhost fixture-parity
```

---

## STOP_IF

- Do not implement `where` in this issue
- Do not extend guard syntax beyond the existing `match` guard model

---

## Close gate

Close when guarded function clauses have positive and negative fixture coverage and the
existing match-guard behavior is unchanged.