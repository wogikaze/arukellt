---
Status: open
Created: 2026-05-16
Updated: 2026-05-16
ID: 629
Track: language-design
Orchestration class: design-ready
Depends on: none
Parent: None
In scope: documentation of current syntax state, gap ledger, frozen scope contract, deferred features list
Out of scope: any implementation, parser/lexer changes, fixture creation
---

# Language Surface Uplift: Baseline and Syntax Contract

---

## Summary

Child issue for #588 Phase 0 — Baseline and Syntax Contract.

This is a **pre-implementation planning issue**. Before any parser/compiler changes in
issues #595-#598 begin, this issue freezes the current syntax state, records the gap
between existing and target surface, and locks down the scope so that implementation
work can proceed without reopening the design on every commit.

This issue produces no code changes. It produces documentation artifacts: a gap ledger
and a scope contract.

---

## Scope

**In scope:**

1. **Record current syntax state** — snapshot of the language surface in `docs/language/spec.md`,
   `docs/language/syntax.md`, and the parser, before any uplift work begins:
   - Function declaration syntax: single-body block form (current)
   - Pattern positions: currently valid only in `match` arms, `let` bindings, and `for` targets
   - `where` keyword status: reserved, parser stub only (`parse_optional_where_clause_stub`)
   - `for` iteration forms: statement-style loops, no expression-level comprehensions
   - Guard availability: only in `match` arms, not on function clauses

2. **Create the gap ledger** — a structured comparison document (or ADR-style note) that
   maps each target surface feature (multi-clause fn, guards, `where`, comprehensions) to:
   - Current behavior
   - Target behavior
   - Desugaring/lowering path
   - Primary compiler paths affected (lexer, parser, resolver, typechecker, corehir)
   - Fixture gaps (what positive/negative tests need to be added)

3. **Freeze the scope contract** — explicitly state the intended scope for the entire
   uplift, matching the four feature areas in #588:
   - Multi-clause `fn` definitions
   - Guards on function clauses
   - Real `where` helper declarations
   - Expression-level comprehensions

4. **Record deferred features** — explicitly list all Haskell-like features that are
   intentionally out of scope for this uplift (see #588 non-goals):
   - Whitespace application / currying-first syntax
   - `do` notation
   - View patterns / pattern synonyms
   - User-defined fixity declarations
   - Backtick infixification
   - Mandatory currying shift

**Out of scope:**
- Any parser, lexer, resolver, typechecker, or emitter changes
- Any fixture creation or modification
- Any documentation changes beyond the gap ledger and scope contract
- Any runtime or ABI changes
- Expanding the uplift scope beyond the four features in #588

---

## Primary paths

- `docs/language/spec.md` (read-only for recording current state)
- `docs/language/syntax.md` (read-only for recording current state)
- `docs/language/maturity-matrix.md` (read-only)
- `src/compiler/parser.ark` (read-only for recording current state of `where` stub)
- `src/compiler/lexer.ark` (read-only for recording `where` tokenization)

## Output artifacts

- Gap ledger document (recommended location: `docs/adr/` or `docs/design/`)
- Scope contract entry in the parent issue #588 (updated to reference all sub-issues)

---

## Upstream / Depends on

None. Phase 0 must be completed before #595-#598 begin implementation work.

## Blocks

- #595 (Multi-clause function definitions — needs frozen scope contract)
- #596 (Function-level guards — needs frozen scope contract)
- #597 (Real `where` clauses — needs frozen scope contract)
- #598 (Expression-level comprehensions — needs frozen scope contract)

---

## Acceptance

1. The current syntax state for function definitions, pattern positions, `where` status,
   `for` forms, and guard positions is recorded in a single place
2. The gap ledger maps each of the four uplift features to current state, target state,
   and affected compiler paths
3. The scope contract explicitly lists what is in scope and what is deferred
4. All deferred Haskell-like features are explicitly documented as out of scope
5. The parent issue #588 is updated to reference all sub-issues with clear dependency ordering

---

## Required verification

```bash
# No code changes — verify by reading the produced artifacts
# The gap ledger and scope contract should be reviewed before #595 starts
```

---

## STOP_IF

- Do not implement any language feature in this issue
- Do not modify compiler source files
- Do not create fixtures
- Do not expand scope beyond what #588 defines
- Do not change runtime behavior or emit output

---

## Close gate

Close when the current state is documented, the gap ledger is complete, the scope
contract is frozen, and the parent #588 issue links to all sub-issues with correct
dependency ordering.
