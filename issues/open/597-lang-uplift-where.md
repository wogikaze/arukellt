# Language Surface Uplift: Real `where` Clauses

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 597
**Parent**: #588
**Depends on**: 595
**Track**: selfhost-frontend / language-design
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #588 Phase 3 — Real `where` Clauses.

Replace the current `parse_optional_where_clause_stub` with real scoped helper declarations
visible across all clauses in a function group.

Example direction:
```ark
fn magnitude_label(p: Point) -> String
| sum == 0 = "origin"
| sum < 10 = "small"
| _ = "large"
where
    sum = p.x * p.x + p.y * p.y
```

The initial restriction: non-recursive value bindings only. Local helper functions and
mutual recursion may be deferred.

---

## Scope

**In scope:**
- Remove `parse_optional_where_clause_stub` as the production behavior
- Define and implement syntax accepted after a function clause group
- Value bindings scoped to all clauses of the group, not visible outside
- Resolver + typechecker integration for where-scope bindings

**Out of scope:**
- Local recursive helper functions inside `where` (deferred)
- `where` in trait bounds / type constraint position (that is a type-system concern, not this issue)
- Comprehensions (#598)
- `where` on non-function items (structs, enums, impls)

---

## Primary paths

- `src/compiler/parser.ark`
- `src/compiler/resolver.ark`
- `src/compiler/typechecker.ark`
- `src/compiler/corehir.ark`
- `tests/fixtures/selfhost/`
- `tests/fixtures/selfhost/parse_where_clause_stub.ark` (must be upgraded or replaced)

## Allowed adjacent paths

- `docs/language/spec.md`
- `docs/language/syntax.md`

---

## Upstream / Depends on

595 (multi-clause fn must exist; `where` attaches to a clause group)

## Blocks

- #599 (docs rollout)

---

## Acceptance

1. `where` is no longer a stub — it parses, resolves, and type-checks value bindings
2. Positive fixtures: shared helper value across guards, helper used in clause body
3. Negative fixtures: duplicate `where` binding, unresolved name in `where`
4. `tests/fixtures/selfhost/parse_where_clause_stub.ark` is replaced by real semantic coverage

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py selfhost fixture-parity
```

---

## STOP_IF

- Do not implement local recursive `where` functions in this issue
- Do not implement `where` as a trait-constraint syntax (that belongs to the type system lane)
- Do not implement comprehensions here

---

## Close gate

Close when `where` value bindings parse, resolve, and typecheck correctly, with positive
and negative fixture coverage, and the old stub fixture is replaced.
