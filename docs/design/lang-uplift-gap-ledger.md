# Language Surface Uplift: Gap Ledger

> **Purpose:** Record the current syntax state (Phase 0 baseline) and map the gap between
> current behavior and target behavior for each of the four uplift features in #588.
>
> This is a **planning artifact** consumed by implementation issues #595-#598.
> It is not an ADR; it freezes the pre-implementation contract.

---

## 1. Current Syntax State (Baseline Snapshot)

Recorded from `docs/language/spec.md`, `docs/language/syntax.md`, `docs/language/maturity-matrix.md`,
`src/compiler/parser.ark`, and `src/compiler/lexer.ark`.

### 1.1 Function Declarations

**Spec** (`docs/language/spec.md` §6.1) describes only the single-body block form:

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- illustrative grammar shape, not a complete standalone program -->
```ark
fn name(param1: T1, param2: T2) -> RetType {
    body
}

pub fn name<T>(param: T) -> T {
    body
}
```

- `pub` makes the function visible to other modules.
- Return type defaults to `()` if omitted.
- Parameters are parsed as typed identifiers (`IDENT ":" type_expr`), NOT as general patterns.

**Parser reality** (`src/compiler/parser.ark`): The parser actually supports **both** forms:

1. **Traditional block-body** (`parse_fn_decl`, line 1827): `fn name<T>(params) -> R { body }` — parameters are typed identifiers.
2. **Clause-style expression-body** (`parse_fn_clause`, line 1613): `fn name(pats...) -> R = expr` — parameters are general patterns.
3. **Multi-clause grouping** (`parse_decl`, line 1506): Consecutive `fn name(...) = ...` with the same name are grouped into one logical function via `desugar_multiclause` (line 1716).

This means the selfhost parser has already advanced beyond what the spec describes.
The clause-style and multi-clause features are **selfhost-only** (no Rust parser equivalent).

### 1.2 Pattern Positions

**Spec** (`docs/language/spec.md` §5, opening paragraph):
> Patterns appear in `match` arms, `let` bindings, and `for` targets.

The spec lists these pattern forms (all labeled stable):
- 5.1 Wildcard (`_`)
- 5.2 Variable Binding (`x`)
- 5.3 Literal Patterns (`42`, `"hello"`, `true`, `'a'`)
- 5.4 Enum Variant Pattern (`Some(x)`, `Shape::Circle(r)`)
- 5.5 Struct Pattern (`Point { x, y }`)
- 5.6 Tuple Pattern (`(a, b, c)`)
- 5.7 Or Pattern (`1 | 2 | 3`)
- 5.8 Match Guards (`x if x > 0`)

**Parser reality**: `parse_pattern` (line 744) handles wildcard, identifier, literal, and
enum variant patterns. Struct patterns (`Point { x, y }`) have `NK_PAT_STRUCT(74)` defined
in `parser_kinds.ark` but are **never produced** by the parser — this is a known gap.

In clause-style fn definitions, params are parsed as patterns (`parse_pattern`, line 1626)
and identifier patterns are converted to `NK_IDENT` for resolver compatibility.

### 1.3 `where` Keyword Status

**Spec** (`docs/language/spec.md` §1.3): `where` is listed as **Reserved (future)**,
alongside `async`, `await`, `dyn`, `type`, `const`, `unsafe`, `extern`, `mod`, `super`, `Self`.

**Lexer** (`src/compiler/lexer.ark`, line 193): `where` is tokenized to `TK_WHERE()` (kind 31).

**Parser** (`src/compiler/parser.ark`, line 731): `parse_optional_where_clause_stub` exists:
- Accepts `where` followed by nothing or a single `_` placeholder.
- Called after generic params on `fn` (line 1856) and `trait` (line 1989).
- This is parser-only stub infrastructure, not a real language feature.

**Fixture** (`tests/fixtures/selfhost/parse_where_clause_stub.ark`): Parse-only fixture proves
the stub accepts `trait Foo where { }` and `fn bar<T>() where _ { }`.

### 1.4 `for` Iteration Forms

**Spec** (`docs/language/spec.md` §4.5): Three forms of statement-style `for`:

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- illustrative loop grammar with placeholder expressions -->
```ark
for i in start..end {     // range: [start, end)
    body
}
for item in values(v) {   // values iterator
    body
}
for item in expr {        // generic iterator
    body
}
```

**No expression-level comprehension** is documented in the spec. The `for` loop is exclusively
a statement form.

**Parser reality**: The selfhost parser DOES have expression-level comprehension support
(`parse_primary`, line 1204):

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- syntax forms only, not standalone checked examples -->
```ark
[expr for x in iter]           // map-only
[expr for x in iter if cond]   // map + filter
```

This produces `NK_COMPREHENSION` nodes. Two fixtures exist:
- `comprehension_map.ark` (map-only)
- `comprehension_map_filter.ark` (map + filter)

The comprehension feature exists in selfhost but is not documented in the spec.

### 1.5 Guard Availability

**Spec** (`docs/language/spec.md` §5.8): Match Guards are listed as a pattern form:

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- match-arm fragment, not standalone source -->
```ark
x if x > 0 => ...
```

**Parser** (`src/compiler/parser.ark`, line 1336): Guards are parsed in `parse_match_expr`:
after pattern (with optional or-pattern via `|`), if `TK_IF` is seen, a guard expression
is parsed.

**Guard availability is currently limited to `match` arms only.** There is no guard syntax
in `parse_fn_clause` (line 1613).

**Fixture**: `tests/fixtures/match_extensions/match_guard.ark` proves guards work in match.

---

## 2. Gap Ledger: Feature-by-Feature

### 2.1 Multi-Clause Function Definitions (#595)

| Aspect | Current State | Target State |
|--------|---------------|--------------|
| Grammar | `parse_fn_clause` exists: `fn name(pats...) -> R = expr`. Grouping via `has_next_clause`. Desugaring via `desugar_multiclause`. | Same, plus struct patterns, full multi-param support, return type checking. |
| Parameter patterns | Literal, identifier, wildcard, enum variant patterns work. Struct patterns (`Point { x, y }`) NOT supported (parser never produces `NK_PAT_STRUCT`). | All match-arm pattern forms available in clause heads. |
| Multi-param | Gated: error "multi-clause functions with multiple parameters are not yet supported". Desugaring code exists but is blocked. | Multi-param clauses supported (tuple pattern desugaring). |
| Return type checking | No cross-clause return type comparison. | All clauses must have compatible return types (or all omit). |
| Type params | Not propagated from first clause to synthetic fn. | Type params from first clause propagated. |
| Fixtures | 4 positive, 2 negative exist (literal, ident, wildcard, single; mixed arity, multi-param). | Add: enum variant, struct pattern, string literal, pub visibility (positive); incompatible return, mixed explicit/implicit (negative). |
| Desugaring location | Parser (`desugar_multiclause`). Downstream passes see desugared form only. | Same — no downstream changes needed. |

**Desugaring path**: `fn_clause_group` → `desugar_multiclause` → single `NK_FN_DECL` with `NK_BLOCK` body containing `NK_MATCH_EXPR`.

**Compiler paths affected**: Parser only. Resolver, typechecker, corehir, MIR see only the desugared form.

### 2.2 Function-Level Guards (#596)

| Aspect | Current State | Target State |
|--------|---------------|--------------|
| Guard in match | EXISTS: `pattern if guard => body` in `parse_match_expr` (line 1336). | Unchanged. |
| Guard on fn clause | NOT AVAILABLE. `parse_fn_clause` has no guard parsing. | `fn name(pats...) -> R | guard = body` — guard after `|` before `=`. |
| Guard expression model | Match guards use `TK_IF` after pattern. | Fn clause guards should reuse same model (guard is a boolean expression after `|` or `if`). |
| Evaluation order | (match) pattern → bind → guard → body. | Same: pattern → bind → guard → body. |
| Fixtures | `match_guard.ark` exists. | Add: literal + guard, enum + guard, struct destructure + guard (positive); guard name not in scope, guard type not bool (negative). |

**Desugaring path**: Guarded clause → match arm with guard in `desugar_multiclause`.

**Compiler paths affected**: Parser (`parse_fn_clause`), possibly typechecker (guard expression type checking).

### 2.3 Real `where` Clauses (#597)

| Aspect | Current State | Target State |
|--------|---------------|--------------|
| Keyword | Reserved. Tokenized as `TK_WHERE` (31). | Same. |
| Parser | `parse_optional_where_clause_stub`: accepts empty or `_` placeholder. Called after fn/trait generic params. | Real parser for `where` value bindings after fn clause group. |
| Semantics | None (stub only). | Scoped helper value bindings visible to all clauses in group. |
| Scope | N/A. | visible to all clauses in group; not visible outside. |
| Initial restriction | N/A. | Non-recursive value bindings only. |
| Fixture | `parse_where_clause_stub.ark` (parse-only). | Replace with real semantic coverage: shared helper, clause body usage (positive); duplicate binding, unresolved name (negative). |

**Desugaring path**: `where` bindings → local `let` bindings in the synthetic block body, before the match expression.

**Compiler paths affected**: Parser (replace stub), resolver (where-scope bindings), typechecker, possibly corehir/MIR.

### 2.4 Expression-Level Comprehensions (#598)

| Aspect | Current State | Target State |
|--------|---------------|--------------|
| Syntax | EXISTS: `[expr for x in iter]` and `[expr for x in iter if cond]` in `parse_primary` (line 1204). | Same — syntax is already frozen. |
| AST node | `NK_COMPREHENSION` with `.text`=loop var, `children[0]`=elem, `children[1]`=iter, `children[2]`=filter (optional). | Same. |
| Typechecker | Loop variable hardcoded to `TY_I32` (line 850 of typechecker.ark). No filter type check. | Infer element type from iterable `Vec<E>`. Check filter is `bool`. Reject non-`Vec` iterable. |
| MIR lowerer | Hardcoded `i32` types (line 3204-3343 of mir_lower.ark). | Generalize to use inferred element types. |
| Fixtures | 2 positive exist (map-only, map+filter). | Add: nested access, as function arg, empty iterable (positive); non-iterable source, filter not bool, unresolved filter, element type mismatch (negative). |
| Spec/docs | NOT documented in spec. | Add to spec §3 as new subsection and grammar appendix. |

**Desugaring path**: `[ELEM for VAR in ITER if FILTER]` → `{ let result: Vec<E> = Vec_new_*(); for VAR in values(ITER) { if FILTER { push(result, ELEM) } }; result }`.

**Compiler paths affected**: Typechecker (element type inference, filter checking), MIR lowerer (type generalization), docs.

---

## 3. Scope Contract (Frozen)

### 3.1 What Is In Scope (Four Features Only)

1. **Multi-clause function definitions** (#595): `fn name(pats...) -> R = expr` grouped by name.
2. **Function-level guards** (#596): Guards on clause heads: `fn name(pats...) -> R | guard = body`.
3. **Real `where` clauses** (#597): Scoped helper value bindings after fn clause groups.
4. **Expression-level comprehensions** (#598): `[expr for x in iter if cond]`.

### 3.2 Design Constraints (Locked)

1. **No runtime model changes** — all new syntax lowers into existing constructs.
2. **No ABI or target changes** — no Wasm backend, Component Model, or stdlib runtime changes.
3. **Existing block-body `fn` syntax remains valid** — additive uplift only.
4. **Prefer explicit desugaring over magic** — each new form has a direct lowering story.
5. **Do not chase full Haskell syntax** — no whitespace application, no mandatory currying shift.

### 3.3 Deferred Features (Explicitly Out of Scope)

The following Haskell-like features are intentionally deferred:
- Whitespace application / currying-first syntax (`f x y`)
- `do` notation
- View patterns / pattern synonyms
- User-defined fixity/precedence declarations
- Backtick infixification of ordinary functions
- Mandatory currying-oriented function syntax
- Module system redesign
- Trait coherence or typeclass-style redesign
- Effect system changes
- Any runtime-only "syntactic" feature with no clear lowering path

### 3.4 Dependency Order

```
#629 (Phase 0: Baseline, done)
  |
  +-- #595 (Phase 1: Multi-clause fn)
  |     |
  |     +-- #596 (Phase 2: Guards on clauses)
  |     |
  |     +-- #597 (Phase 3: Real where)
  |
  +-- #598 (Phase 4: Comprehensions — independent of #595-#597)
        |
#599 (Phase 5: Docs/Rollout — waits for all four features)
```

---

## 4. Verification Contract

- No code changes in Phase 0.
- Verification for Phase 0: review of this gap ledger and scope contract.
- Implementation phases (#595-#598) must pass:

  ```bash
  python3 scripts/manager.py verify quick
  python3 scripts/manager.py verify fixtures
  python3 scripts/manager.py selfhost fixture-parity
  ```

- Phase 5 (#599) additionally requires:

  ```bash
  python3 scripts/gen/generate-docs.py
  python3 scripts/manager.py docs check
  ```
