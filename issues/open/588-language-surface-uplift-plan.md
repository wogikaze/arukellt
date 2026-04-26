# Language Surface Uplift Plan (Operational Guide)

> **Status:** Implementation Guide — ready for subissue decomposition with verification checkpoints
> **For agentic workers:** Do not implement this umbrella directly. Split into focused parser, resolver, typechecker, lowering, docs, and fixture subissues before execution.

> ⚠️ **DO NOT IMPLEMENT DIRECTLY.** This is an operational guide umbrella. Dispatch child issues:
> - **#595** `595-lang-uplift-multiclauses.md` — multi-clause / multi-arm patterns (implementation-ready)
> - **#596** `596-lang-uplift-guards.md` — match guards (depends: 595)
> - **#597** `597-lang-uplift-where.md` — `where` clause sugar (depends: 595)
> - **#598** `598-lang-uplift-comprehensions.md` — comprehension expressions (implementation-ready)
> - **#599** `599-lang-uplift-docs-rollout.md` — docs + rollout (depends: 595/596/597/598, closes #588)

**Goal:** Raise Arukellt from a Rust-like surface with strong `match` into a pattern-first definition surface with multi-clause functions, function-level guards, real `where` clauses, and expression-level comprehensions.

**Implementation target:** Use Ark (`src/compiler/*.ark`) instead of Rust crates (`crates/*`) per #529 100% selfhost transition plan.

**Work Streams (DO NOT MIX):**
1. Frontend syntax: `src/compiler/lexer.ark`, `src/compiler/parser.ark`
2. Name binding and type semantics: `src/compiler/resolver.ark`, `src/compiler/typechecker.ark`
3. Lowering: `src/compiler/corehir.ark`, `src/compiler/mir.ark`
4. Verification: `tests/fixtures/`, selfhost fixture parity, negative syntax fixtures
5. Documentation: `docs/language/*`, `docs/data/language-doc-classifications.toml`

**Key Constraint:** First goal is **NOT** “copy Haskell wholesale”. First goal is **“make existing Arukellt pattern, guard, and block semantics shape the definition surface without changing the runtime model.”**

**Issue metadata:** ID 588; status open; created 2026-04-22; updated 2026-04-22; track language-design; orchestration class design-ready; upstream #529.

## Summary

Arukellt already has a strong modern syntax core: `match` with guards / or-patterns / struct patterns, `let` destructuring, `for` patterns, `struct` update, generics, traits / impls, and optional semicolons.  
However, the *writing feel* is still primarily Rust-shaped: branching power is concentrated inside `match`, function definitions are single-body block forms, and `where` is only reserved / stubbed.

This issue proposes the next syntax stage: **raise Arukellt from “Rust-like surface with strong `match`” to “pattern-first definition surface”** without changing the execution model, ABI, or Wasm-oriented implementation strategy.

The target is **not full Haskell syntax**. The target is a focused uplift that adds the parts most likely to improve readability and declarative expression while preserving Arukellt’s current strengths:

1. **multi-clause function definitions**
2. **function-level guards**
3. **real `where` clauses**
4. **expression-level comprehensions**

This is the smallest next step that meaningfully closes the gap between current Arukellt and a more equation-oriented / pattern-oriented surface.

---

## Repo evidence

### Current strengths already present

- `docs/language/spec.md` defines patterns as valid in `match` arms, `let` bindings, and `for` targets.
- `docs/language/spec.md` and `docs/language/maturity-matrix.md` mark `match` guards, or-patterns, struct patterns, tuple patterns, arrays, tuples, closures, generics, and struct update as implemented.
- `tests/fixtures/match_extensions/` contains working fixtures for:
  - `match_guard.ark`
  - `struct_pattern.ark`
  - `struct_field_update.ark`
- `docs/current-state.md` reports fixture harness health at `641 passed / 0 failed / 28 skipped`, with `19/19` verification checks passing.

### Current syntax gap

- `docs/language/spec.md` keyword table reserves `where` for future use.
- `src/compiler/lexer.ark` tokenizes `where` as `TK_WHERE()`.
- `src/compiler/parser.ark` contains `parse_optional_where_clause_stub(p)` and only accepts an empty clause or `_` placeholder after generic params on `fn` / `trait`.
- `tests/fixtures/selfhost/parse_where_clause_stub.ark` proves that `where` is currently parse-only stub infrastructure, not a real language feature.
- `docs/language/spec.md` still defines function declarations in the single-body form:

```ark
fn name(param1: T1, param2: T2) -> RetType {
    body
}
```

- In `src/compiler/parser.ark`, function parameters are parsed as identifiers with optional type annotations; they are **not** general patterns.
- There is no expression-level comprehension syntax in `docs/language/spec.md`, `docs/language/syntax.md`, or the current fixture set.

---

## Problem statement

Arukellt has strong *local* pattern matching, but it does not yet have strong *definition-oriented* pattern syntax.

Today, the language encourages this shape:

```ark
fn classify(x: i32) -> String {
    match x {
        0 => "zero",
        n if n > 0 => "pos",
        _ => "neg",
    }
}
```

That is already good. But it still pushes the user toward:
- a single function body
- an inner `match`
- helper bindings pushed upward into `let`
- collection transforms expressed mostly through statement-style loops

The missing step is to let patterns and guards shape the definition surface itself.

The intended uplift is:

- **patterns move from “inside `match`” toward “at the function boundary”**
- **shared helper computations move into `where`**
- **simple collection construction moves into expression-level comprehensions**
- **the language becomes more declarative without changing runtime behavior**

This gives Arukellt more of the “definition reads like a specification” quality associated with Haskell-style syntax, while staying compatible with the current parser architecture, selfhost implementation path, and Wasm target story.

---

## Current state vs target state

| Area | Current Arukellt | Target after this issue |
|------|------------------|-------------------------|
| Function definitions | One block body per `fn` | Multiple clauses per function group |
| Patterns at definition boundary | Not available in function params | Available in clause heads |
| Guards | Only in `match` arms | Also available on function clauses |
| `where` | Reserved keyword + parser stub | Real scoped helper declarations |
| Collection query syntax | Statement-style `for` loops only | Expression-level comprehension |
| Runtime / ABI | Current behavior | Unchanged |
| Module / import system | Separate topic (#123 etc.) | Out of scope |
| Effect notation / `do` | Not present | Out of scope |
| Operator culture / fixity | Minimal and Rust-shaped | Out of scope for this stage |
| View patterns / pattern synonyms | Not present | Explicitly deferred |

---

## Design constraints

1. **No runtime model changes**  
   All new syntax in this issue must lower into existing constructs (`match`, block expressions, local bindings, loops / builders, existing pattern engine).

2. **No ABI or target changes**  
   This issue must not depend on Wasm backend redesign, Component Model work, or stdlib runtime changes.

3. **Keep current block-body `fn` syntax valid**  
   This is an additive surface uplift, not a syntax replacement.

4. **Prefer explicit desugaring over magic**  
   Each new surface form must have a direct lowering story that can be reasoned about in parser / AST / typechecker / CoreHIR terms.

5. **Do not chase full Haskell syntax yet**  
   No whitespace application, no mandatory currying shift, no user-defined operator fixity system, no `do` notation in this issue.

---

## Proposed syntax direction

### A. Multi-clause function definitions

Preferred direction: allow a function to be written as multiple clauses sharing the same name.

Example direction:

```ark
fn classify(0) -> String = "zero"
fn classify(n) -> String | n > 0 = "pos"
fn classify(_) -> String = "neg"
```

Equivalent current form:

```ark
fn classify(x: i32) -> String {
    match x {
        0 => "zero",
        n if n > 0 => "pos",
        _ => "neg",
    }
}
```

Key property: this is **surface-level sugar over existing `match` + guard behavior**.

### B. Real `where` clauses

Preferred direction: allow helper definitions that scope over a function clause group.

Example direction:

```ark
fn magnitude_label(p: Point) -> String
| sum == 0 = "origin"
| sum < 10 = "small"
| _ = "large"
where
    sum = p.x * p.x + p.y * p.y
```

This should solve two current problems:
- helper values shared across guarded clauses
- keeping the main decision surface visually primary

The existing `where` parser stub should be replaced by real syntax and semantics.

### C. Expression-level comprehensions

Preferred direction: add a lightweight collection-construction form that complements statement-style `for`.

Example direction:

```ark
let ys = [x * 2 for x in xs if x > 0]
```

Possible extended direction later:

```ark
let pairs = [(x, y) for x in xs for y in ys if x != y]
```

This feature should be explicitly constrained to **desugar into existing iteration / builder constructs**.  
It is a syntax improvement, not a new evaluation strategy.

---

## Non-goals

- Full Haskell whitespace application (`f x y`)
- Mandatory currying-oriented function syntax
- `do` notation
- User-defined operator precedence / fixity declarations
- Backtick infixification of ordinary functions
- View patterns
- Pattern synonyms
- Module system redesign
- Trait coherence or typeclass-style redesign
- Effect system changes
- Any runtime-only “syntactic” feature with no clear lowering path

---

## Why these features first

This issue intentionally prioritizes the Haskell-derived parts that:

- improve readability the most
- reuse existing Arukellt machinery
- do not fight the Wasm target
- do not require a new execution model
- make the language look and feel more declarative in user code

These features also match the repo’s current shape:

- patterns are already strong
- `where` is already reserved and partially plumbed
- `match` guards already exist
- `for` already exists as a statement form
- selfhost parser / lexer / spec / fixture infrastructure already exists

In other words: the repo already contains the *pieces*; this issue is about making them shape the surface of definitions instead of remaining isolated features.

---

## Execution phases

### Phase 0: Baseline and syntax contract

**Purpose:** Freeze the current gap before implementation.

**Tasks:**
- Record the current definition of:
  - function declaration syntax
  - pattern positions
  - `where` keyword status
  - `for` iteration forms
- Add a design note section to this issue (or ADR draft) that states the exact intended scope:
  - multi-clause `fn`
  - guards on clauses
  - real `where`
  - comprehension
- Explicitly record all deferred Haskell-like features as out of scope.

**Exit condition:**
- The target surface and non-goals are frozen enough that parser work can proceed without reopening the issue scope every day.

---

### Phase 1: Multi-clause function definitions

**Goal:** Let function definitions be written as grouped clauses rather than only a single block body.

**Required work:**
- Extend grammar so repeated `fn <name>(...) ...` clauses can form one logical function group.
- Allow clause-head patterns in parameter position.
- Preserve existing single-body block form.
- Define grouping rules:
  - same function name
  - same arity
  - compatible return annotation policy
- Define desugaring into current internal representation.

**Semantic requirements:**
- Clause ordering is stable and source-visible.
- Exhaustiveness / overlap policy is documented.
- Diagnostics for mixed arity / mixed incompatible signatures are explicit.

**Suggested initial restriction:**
- First implementation may limit clause patterns to the same pattern subset already supported in `match`.

**Verification (mandatory):**
- Positive fixtures:
  - literal pattern clauses
  - wildcard / identifier clauses
  - enum variant clauses
  - tuple / struct clauses
- Negative fixtures:
  - mixed arity in one group
  - incompatible return annotations
  - unreachable later clause (if overlap warnings/errors are implemented)

**Phase 1 Exit Condition:**
- A user can rewrite a simple `match`-based classifier as multi-clause `fn` syntax.
- The new form is documented in `docs/language/spec.md` and `docs/language/syntax.md` as provisional or stable according to implementation status.

---

### Phase 2: Function-level guards

**Goal:** Allow guarded clauses outside `match`.

**Required work:**
- Reuse the existing guard expression model already supported in `match`.
- Define syntax for guarded clauses.
- Define ordering relative to pattern matching:
  1. match clause head pattern
  2. establish bindings
  3. evaluate guard
  4. choose body or continue

**Verification (mandatory):**
- Positive fixtures:
  - literal + guard
  - enum + guard
  - struct destructuring + guard
- Negative fixtures:
  - guard name not in scope
  - guard type is not `bool`

**Phase 2 Exit Condition:**
- A user can express the common “equation + guard” style without dropping into an inner `match`.

---

### Phase 3: Real `where` clauses

**Goal:** Replace the current parse stub with real scoped helper declarations.

**Required work:**
- Remove `parse_optional_where_clause_stub(p)` as the long-term behavior.
- Define the syntax accepted after a function clause group.
- Define what declarations are allowed inside `where`:
  - value bindings
  - possibly local helper functions
- Define scope clearly:
  - visible to all clauses in the group
  - not visible outside the group
- Decide whether `where` is expression-only, declaration-only, or a limited local item block.

**Suggested initial restriction:**
- First implementation may support only non-recursive value bindings.
- Local functions and mutual recursion may be deferred.

**Verification (mandatory):**
- Positive fixtures:
  - shared helper value across guards
  - helper used in clause body
- Negative fixtures:
  - duplicate `where` binding
  - unresolved name in `where`
  - illegal recursive reference if recursion is deferred

**Phase 3 Exit Condition:**
- `where` is no longer a reserved/stub-only surface.
- `tests/fixtures/selfhost/parse_where_clause_stub.ark` is replaced or upgraded to semantic coverage.

---

### Phase 4: Expression-level comprehensions

**Goal:** Add a declarative collection-construction form that complements `for`.

**Required work:**
- Choose and freeze canonical syntax.
- Support at least:
  - one generator
  - one optional filter
- Define element type inference and resulting collection type.
- Define lowering to existing collection / iteration machinery.

**Suggested initial restriction:**
- Start with array / Vec-style construction only.
- Defer nested generators or local declarations if needed.

**Verification (mandatory):**
- Positive fixtures:
  - map-only comprehension
  - map + filter comprehension
  - pattern destructuring in generator target if supported
- Negative fixtures:
  - non-iterable source
  - filter not `bool`

**Phase 4 Exit Condition:**
- Basic “build transformed collection from iterable” code can be written as a single expression.

---

### Phase 5: Canonical surface, docs, and migration guidance

**Goal:** Make the uplift usable and teachable.

**Required work:**
- Update:
  - `docs/language/spec.md`
  - `docs/language/syntax.md`
  - `docs/language/syntax-v1-preview.md` (if still needed as transitional landing zone)
  - `docs/language/maturity-matrix.md`
- Add fixture-backed examples for the new surface.
- State clearly when clause syntax is preferred over explicit `match`.
- Document the retained canonical block-body form and interoperability between styles.

**Verification (mandatory):**
- docs examples compile or parse through fixture-backed checks
- maturity labels and docs generation stay in sync
- no drift between guide / spec / fixture examples

**Phase 5 Exit Condition:**
- The new syntax is not merely implemented; it is part of the teachable language surface.

---

## Acceptance

- [ ] The syntax gap between current Arukellt and the target “pattern-first definition surface” is documented in one place
- [ ] Multi-clause function definitions are supported and lowered deterministically
- [ ] Function-level guards are supported
- [ ] `where` is implemented as real syntax, not a parser stub
- [ ] At least one expression-level comprehension form is implemented
- [ ] Existing block-body `fn` syntax remains supported
- [ ] `docs/language/spec.md` and `docs/language/syntax.md` describe the new syntax
- [ ] Fixture coverage includes both positive and negative cases for each added surface feature
- [ ] The implementation is fully in the selfhost compiler (`src/compiler/*.ark`) and fits the #529 selfhost direction

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py selfhost fixture-parity
```

If selfhost-specific syntax coverage is split into parse-only and semantic fixtures, both categories must pass.

---

## Primary paths

- `src/compiler/lexer.ark`
- `src/compiler/parser.ark`
- `src/compiler/resolver.ark`
- `src/compiler/typechecker.ark`
- `src/compiler/corehir.ark`
- `src/compiler/mir.ark`
- `docs/language/spec.md`
- `docs/language/syntax.md`
- `docs/language/syntax-v1-preview.md`
- `docs/language/maturity-matrix.md`
- `tests/fixtures/`
- `docs/data/language-doc-classifications.toml`

---

## Follow-up issues to split out after plan approval

1. Multi-clause function definitions and grouping rules
2. Function-level guards
3. Real `where` clause semantics
4. Comprehension syntax and lowering
5. Docs / fixtures / migration guidance rollout

---

## Deferred after this issue

These may still be desirable later, but they are **not** the right next step for Arukellt’s current stage:

- `do` notation
- backticks / function-operator interchange
- user-defined fixity declarations
- view patterns
- pattern synonyms
- whitespace application / currying-first syntax

These are intentionally deferred because their implementation cost, semantic weight, or ecosystem consequences are larger than the “one stage up” language uplift targeted here.

---

## Close gate

Close this issue only when Arukellt can credibly be described as:

> a Wasm-first language with Rust-like operational clarity and a more Haskell-like definition surface

—not because it copied Haskell wholesale, but because function definitions, guards, local helper bindings, and collection-building expressions became more declarative in a way that is visible in everyday code.
