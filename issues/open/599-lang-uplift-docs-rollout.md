---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 599
Track: docs / language-design
Orchestration class: blocked-by-upstream
Depends on: 595, 596, 597, 598
Parent: None
In scope: 
Out of scope: 
# Language Surface Uplift: Docs, Fixtures, and Rollout
---
# Language Surface Uplift: Docs, Fixtures, and Rollout

---

## Summary

Child issue for #588 Phase 5 — Canonical Surface, Docs, and Migration Guidance.

After multi-clause functions (#595), guards (#596), `where` clauses (#597), and
comprehensions (#598) are implemented, this issue makes the uplift usable and teachable
by updating all relevant documentation and ensuring fixture-backed examples are consistent
with the final spec.

---

## Scope

**In scope:**
- Update `docs/language/spec.md` with all four new syntax features
- Update `docs/language/syntax.md` with all four new syntax features
- Update `docs/language/maturity-matrix.md` with stability labels
- Update `docs/language/syntax-v1-preview.md` if still needed as a transitional zone
- Add fixture-backed examples for each new surface
- State when clause syntax is preferred over explicit `match`
- Document retained canonical block-body form and interoperability between styles
- Regenerate docs

**Out of scope:**
- Implementing any of the four syntax features (those are #595-#598)
- Adding features beyond what #595-#598 implemented
- Full Haskell-style syntax documentation

---

## Primary paths

- `docs/language/spec.md`
- `docs/language/syntax.md`
- `docs/language/syntax-v1-preview.md`
- `docs/language/maturity-matrix.md`
- `docs/data/language-doc-classifications.toml`
- `tests/fixtures/` (example fixtures referenced from docs)

---

## Upstream / Depends on

595, 596, 597, 598 — all implementation issues must be complete first.

## Blocks

None (this closes the #588 umbrella)

---

## Acceptance

1. Docs examples for all four features compile / parse through fixture-backed checks
2. `python3 scripts/gen/generate-docs.py` runs without drift warnings
3. Maturity labels in `docs/language/maturity-matrix.md` and generated docs are consistent
4. No drift between guide / spec / fixture examples

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py selfhost fixture-parity
python3 scripts/gen/generate-docs.py
python scripts/manager.py docs check
```

---

## STOP_IF

- Do not implement language features in this issue
- Do not add syntax features that were explicitly deferred in #588 non-goals

---

## Close gate

This issue closes the #588 umbrella. Close when all four uplift docs are updated,
generated docs pass the consistency check, and fixture-backed examples are accurate.