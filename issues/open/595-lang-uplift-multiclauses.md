---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 595
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Language Surface Uplift: Multi-Clause Function Definitions
**Parent**: #588
**Depends on**: —
**Track**: selfhost-frontend / language-design
**Orchestration class**: design-ready

---

## Summary

Child issue for #588 Phase 1 — Multi-Clause Function Definitions.

Add support for defining a function as a group of pattern-matching clauses rather than
requiring a single block body. The new syntax is surface sugar that desugars into existing
`match` + guard behavior. Existing block-body `fn` syntax must remain supported.

Example direction:
```ark
fn classify(0) -> String = "zero"
fn classify(n) -> String | n > 0 = "pos"
fn classify(_) -> String = "neg"
```

---

## Scope

**In scope:**
- Grammar extension: repeated `fn <name>(...)` clauses forming one logical function group
- Clause-head patterns in parameter position (subset supported by current `match`)
- Grouping rules: same name, same arity, compatible return annotation
- Desugaring definition into current internal representation
- Phase 0 gap ledger for this feature (document current state before implementation)

**Out of scope:**
- Guards on clause heads (that is #596)
- `where` clauses (that is #597)
- Comprehensions (that is #598)
- Mixed-arity or variadic functions
- View patterns, pattern synonyms

---

## Primary paths

- `src/compiler/lexer.ark`
- `src/compiler/parser.ark`
- `src/compiler/resolver.ark`
- `src/compiler/typechecker.ark`
- `src/compiler/corehir.ark`
- `tests/fixtures/selfhost/` (positive + negative fixtures)

## Allowed adjacent paths

- `docs/language/syntax.md` (provisional notation update)
- `docs/language/spec.md` (provisional section)
- `docs/data/language-doc-classifications.toml`

---

## Upstream / Depends on

None. Can start once Phase 0 gap ledger is written.
This issue is compatible with the selfhost path (#529).

## Blocks

- #596 (guards on clauses require clause syntax to exist first)
- #599 (docs rollout covers all four uplift features)

---

## Acceptance

1. A user can define a simple classifier as grouped `fn` clauses without writing an inner `match`
2. Positive fixtures pass: literal-pattern clauses, wildcard clauses, enum-variant clauses,
   tuple/struct clauses
3. Negative fixtures pass: mixed arity in one group, incompatible return annotations
4. Existing single-block-body `fn` syntax compiles unchanged

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py selfhost fixture-parity
```

---

## STOP_IF

- Do not implement guards in this issue (that is #596)
- Do not implement `where` in this issue (that is #597)
- Do not change runtime ABI or emitter output format
- Do not merge clause semantics with `trait impl` dispatch

---

## Close gate

Close when: positive and negative fixtures for clause-based functions pass, existing fixtures
do not regress, and `docs/language/syntax.md` has a provisional section for the new syntax.