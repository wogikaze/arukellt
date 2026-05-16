---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 595
Track: selfhost-frontend / language-design
Orchestration class: design-ready
Depends on: 629
Parent: None
In scope: 
Out of scope: 
Close when: positive and negative fixtures for clause-based functions pass, existing fixtures
---

1. Positive fixtures pass: literal-pattern clauses, wildcard clauses, enum-variant clauses,
2. Negative fixtures pass: mixed arity in one group, incompatible return annotations

# Language Surface Uplift: Multi-Clause Function Definitions

- Grammar extension: "repeated `fn <name>(...)` clauses forming one logical function group"
- Grouping rules: same name, same arity, compatible return annotation

# Language Surface Uplift: Multi-Clause Function Definitions

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

629 (Phase 0 gap ledger must be written before clause syntax work begins).
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

## Design Spec

### 1. Detailed Grammar Specification

#### 1.1 Current Grammar (Baseline)

From `docs/language/spec.md` Appendix A, the current function definition grammar:

```ebnf
fn_def      = "fn" IDENT type_params? "(" param_list? ")"
              ("->" type_expr)? block ;
param       = IDENT ":" type_expr ;
```

Existing fn parameters are typed identifiers. No patterns in parameter position.

#### 1.2 Extended Grammar (Proposed)

Add a new production for multi-clause function definitions alongside the existing `fn_def`:

```ebnf
item        = ("pub")? ( fn_def | fn_clause_group | struct_def | enum_def
                        | trait_def | impl_block ) ;

fn_def      = "fn" IDENT type_params? "(" param_list? ")"
              ("->" type_expr)? block ;
              (* unchanged -- single body form *)

fn_clause_group
            = fn_clause+ ;
              (* one or more consecutive fn_clause with same name *)

fn_clause   = "fn" IDENT type_params? "(" clause_params? ")"
              ("->" type_expr)? "=" expr ;
              (* mandatory `= expr` body -- no block form *)

clause_params = clause_param ("," clause_param)* ","? ;
clause_param  = pattern ;
              (* patterns instead of typed identifiers *)

pattern     = "_"
            | IDENT
            | INT_LIT | FLOAT_LIT | STRING_LIT | CHAR_LIT | BOOL_LIT
            | IDENT "::" IDENT pattern_args?
            | IDENT "{" pattern_fields "}"
            | "(" pattern ("," pattern)* ")"
            | pattern ("|" pattern)+ ;
            (* unchanged -- shared with match arms *)

pattern_args  = "(" pattern ("," pattern)* ")" ;
pattern_fields= (IDENT (":" pattern)?) ("," IDENT (":" pattern)?)* ;
```

#### 1.3 Clause Grouping Rules

A `fn_clause_group` is a maximal contiguous sequence of `fn_clause` entries satisfying all of:

1. **Same name**: Every clause has the same `IDENT` (checked by `has_next_clause`).
2. **Same arity**: Every clause has the same number of `clause_params`. Mixed arity is a **compile-time error**.
3. **Compatible return annotation**: All clauses must have the **same** `-> type_expr`. Omission of `-> type_expr` is allowed on all clauses (implicit `-> ()`) but is not allowed to mix with clauses that have an explicit return type. Incompatible/mixed annotations are a **compile-time error**.
4. **Contiguous**: Clauses must be consecutive in source. An intervening non-`fn` item or a `fn` with a different name terminates the group.

#### 1.4 Syntax at a Glance

```ark
// Single arity, literal and wildcard patterns
fn classify(0) -> String = "zero"
fn classify(1) -> String = "one"
fn classify(_) -> String = "other"

// Single arity, identifier binding (match-all)
fn negate(true) -> bool = false
fn negate(false) -> bool = true

// Single arity, enum variant patterns
fn describe(Option::Some(v)) -> String = concat(String_from("some: "), to_string(v))
fn describe(None) -> String = String_from("none")

// Single arity, struct patterns (must include all fields that the pattern destructures)
fn origin(Point { x: 0, y: 0 }) -> bool = true
fn origin(_) -> bool = false

// Clause group with type parameters
fn first<T>(x: T, _) -> T = x
```

#### 1.5 What is NOT in Scope for This Grammar

- Guards on clause heads (`| expr` after clause params but before `=`) -- #596
- `where` clauses after the group -- #597
- Or-patterns at the clause level (though or-patterns within a single clause param are valid via existing `pattern` grammar)

---

### 2. Desugaring Specification

Every `fn_clause_group` is desugared at parse time into a single `fn_def` with a synthetic block body containing a `match` expression. The desugaring is deterministic and preserves semantics.

#### 2.1 General Scheme

```ark
// Source:
fn foo(P1_1, P1_2, ...) -> R = body1
fn foo(P2_1, P2_2, ...) -> R = body2
fn foo(P3_1, P3_2, ...) -> R = body3

// Desugared:
fn foo(__arg0, __arg1, ...) -> R {
    match (__arg0, __arg1, ...) {
        (P1_1, P1_2, ...) => body1,
        (P2_1, P2_2, ...) => body2,
        (P3_1, P3_2, ...) => body3,
    }
}
```

#### 2.2 Single-Parameter Special Case

When arity is 1, the tuple wrapper is elided and the scrutinee is a bare identifier:

```ark
// Source:
fn classify(0) -> String = "zero"
fn classify(n) -> String = "pos"
fn classify(_) -> String = "neg"

// Desugared:
fn classify(__arg0) -> String {
    classify(__arg0) {
        0 => "zero",
        n => "pos",
        _ => "neg",
    }
}
```

#### 2.3 Pattern Conversion Rules

Each clause parameter (parsed as a `pattern`) becomes a match-arm pattern in the desugared match body:

| Clause param kind | Match arm pattern kind | Notes |
|---|---|---|
| `NK_PAT_WILDCARD` (`_`) | `NK_PAT_WILDCARD` | No conversion needed |
| `NK_PAT_LITERAL` (42, "hi", true) | `NK_PAT_LITERAL` | No conversion needed |
| `NK_PAT_IDENT` (`n`) | `NK_PAT_IDENT` during parse → converted to `NK_IDENT` for param registration, then `clause_param_to_arm_pattern()` converts back to `NK_PAT_IDENT` for the match arm |
| `NK_PAT_VARIANT` (`Option::Some(v)`) | `NK_PAT_VARIANT` | No conversion needed |
| `NK_PAT_STRUCT` (`Point { x, y }`) | `NK_PAT_STRUCT` | Needs parser to produce this node kind (see gap ledger) |
| `NK_PAT_TUPLE` (`(a, b)`) | `NK_PAT_TUPLE` | Already used for multi-param desugaring internally |

#### 2.4 Visibility Propagation

If the first clause is `pub`, the desugared function is `pub`. All clauses in the group share the same visibility -- currently enforced by applying `is_pub` from the first item to the synthetic result.

#### 2.5 Return Type Handling

The return type annotation (if present) is taken from the **first clause** and applied to the synthetic function. All clauses in the group must carry the same `-> type_expr` -- this is a compile-time check. If the first clause omits the return type, all clauses must omit it.

#### 2.6 Type Parameter Handling

Type parameters declared on the first clause are propagated to the synthetic function. All clauses in the group must have identical type parameter lists (checked at grouping time).

#### 2.7 Desugaring Location

Desugaring happens in the parser, in the existing `desugar_multiclause` function (currently at `src/compiler/parser.ark`, line 1716). The parser produces a single `NK_FN_DECL` node; downstream passes (resolver, typechecker, corehir, MIR) see only the desugared form and require no changes.

---

### 3. Positive Fixture Specifications

Each positive fixture must compile and run, producing the expected output (via `.expected` file).

#### 3.1 Literal Pattern Clauses

| # | File | Description |
|---|---|---|
| P1 | `fn_multiclause_literal.ark` | **Exists**. Integer literal clauses with wildcard catch-all. |

**Expected behavior**: `classify(0)` → `"zero"`, `classify(1)` → `"one"`, `classify(2)` → `"other"`, `classify(-1)` → `"other"`.

#### 3.2 Wildcard / Identifier Clauses

| # | File | Description |
|---|---|---|
| P2 | `fn_multiclause_wildcard.ark` | **Exists**. Wildcard catch-all with literal. |
| P3 | `fn_multiclause_ident.ark` | **Exists**. Boolean literal clauses with identifier binding. |

**Expected behavior** (P2): `answer(42)` → `"life"`, `answer(0)` → `"unknown"`, `answer(99)` → `"unknown"`.
**Expected behavior** (P3): `negate(true)` → `false`, `negate(false)` → `true`.

#### 3.3 Enum Variant Clauses (NEW)

| # | File | Description |
|---|---|---|
| P4 | `fn_multiclause_enum_variant.ark` | Enum variant patterns in clause heads with named enum. |

```ark
// tests/fixtures/selfhost/fn_multiclause_enum_variant.ark
use std::host::stdio

enum Opt {
    Some(i32),
    None,
}

fn unwrap_or_zero(Opt::Some(v)) -> i32 = v
fn unwrap_or_zero(Opt::None) -> i32 = 0

fn main() {
    stdio::println(i32_to_string(unwrap_or_zero(Opt::Some(42))))
    stdio::println(i32_to_string(unwrap_or_zero(Opt::None)))
}
```

**Expected output**: `42`, `0`.

#### 3.4 Struct Pattern Clauses (NEW)

| # | File | Description |
|---|---|---|
| P5 | `fn_multiclause_struct_pattern.ark` | Struct destructuring in clause heads. |

```ark
// tests/fixtures/selfhost/fn_multiclause_struct_pattern.ark
use std::host::stdio

struct Point { x: i32, y: i32 }

fn is_origin(Point { x: 0, y: 0 }) -> bool = true
fn is_origin(_) -> bool = false

fn main() {
    stdio::println(bool_to_string(is_origin(Point { x: 0, y: 0 })))
    stdio::println(bool_to_string(is_origin(Point { x: 1, y: 2 })))
}
```

**Expected output**: `true`, `false`.

#### 3.5 Single Clause (Degenerate Case)

| # | File | Description |
|---|---|---|
| P6 | `fn_multiclause_single.ark` | **Exists**. A single expression-bodied fn (degenerate clause group). |

**Note**: This fixture tests that `fn foo(x: T) -> R = expr` (a single clause with typed identifier params) still works as before.

#### 3.6 String Literal Clauses (NEW)

| # | File | Description |
|---|---|---|
| P7 | `fn_multiclause_string_literal.ark` | String literal patterns in clause heads. |

```ark
// tests/fixtures/selfhost/fn_multiclause_string_literal.ark
use std::host::stdio

fn greet("hello") -> String = String_from("hi")
fn greet("bye") -> String = String_from("goodbye")
fn greet(_) -> String = String_from("huh?")

fn main() {
    stdio::println(greet(String_from("hello")))
    stdio::println(greet(String_from("bye")))
    stdio::println(greet(String_from("other")))
}
```

**Expected output**: `hi`, `goodbye`, `huh?`.

#### 3.7 Pub Visibility Clauses (NEW)

| # | File | Description |
|---|---|---|
| P8 | `fn_multiclause_pub.ark` | Public visibility with multi-clause syntax. |

```ark
// tests/fixtures/selfhost/fn_multiclause_pub.ark
pub fn is_zero(0) -> bool = true
pub fn is_zero(_) -> bool = false

fn main() {
    // The fn itself is pub; internal usage works
}
```

**Expected output**: (none, or compiles successfully as parse-only fixture if side-effect-free.)

---

### 4. Negative Fixture Specifications

Each negative fixture must produce a deterministic compile-time error matching an expected diagnostic pattern (via `.diag` file).

#### 4.1 Mixed Arity (EXISTS)

| # | File | Description |
|---|---|---|
| N1 | `fn_multiclause_mixed_arity.ark` | **Exists**. Clauses with different parameter counts in the same group. |

**Diagnostic regex**: `clause has \d+ parameters but previous clauses have \d+`

#### 4.2 Multi-Parameter Clauses (EXISTS)

| # | File | Description |
|---|---|---|
| N2 | `fn_multiclause_multi_param.ark` | **Exists**. Multi-param clauses (currently not supported). |

**Diagnostic**: `multi-clause functions with multiple parameters are not yet supported`

#### 4.3 Incompatible Return Annotations (NEW)

| # | File | Description |
|---|---|---|
| N3 | `fn_multiclause_incompatible_return.ark` | Clauses with differing explicit return types. |

```ark
// tests/fixtures/selfhost/fn_multiclause_incompatible_return.ark
// Error: clauses in the same group have incompatible return type annotations
fn bad(0) -> String = String_from("zero")
fn bad(1) -> i32 = 1
//       ^^ error: return type mismatch in multi-clause group

fn main() {}
```

**Diagnostic** (proposed): `clause return type mismatch in multi-clause group for 'bad'`

#### 4.4 Mixed Explicit and Implicit Return (NEW)

| # | File | Description |
|---|---|---|
| N4 | `fn_multiclause_mixed_return.ark` | Some clauses have `-> R`, others omit it. |

```ark
// tests/fixtures/selfhost/fn_multiclause_mixed_return.ark
// Error: clauses mix explicit return annotation and implicit ()
fn bad(0) -> String = String_from("zero")
fn bad(_) = String_from("other")
//       ^^ error: clause has no return type but previous clauses specify one

fn main() {}
```

**Diagnostic** (proposed): `clause return type mismatch: group mixes explicit and implicit return types`

#### 4.5 Name Mismatch / Non-Contiguous Group (NEW)

| # | File | Description |
|---|---|---|
| N5 | `fn_multiclause_name_mismatch.ark` | Clause sequence interrupted by different-name fn. |

This is actually a **positive** behavior test -- different-name fns should separate groups correctly:

```ark
// tests/fixtures/selfhost/fn_multiclause_name_mismatch.ark
// No error expected -- these form two separate single-clause groups
fn foo(0) -> i32 = 0
fn bar(1) -> i32 = 1

fn main() {
    // separate groups, no error
}
```

(Consider if this adds value as a positive edge-case fixture instead.)

#### 4.6 Duplicate Clause / Unreachable Warning (Informational)

Overlap detection (e.g., two wildcard clauses, or a literal clause after a catch-all) is **documented for future work** but is **not required** for Phase 1. The match engine's existing behavior (first-match-wins) applies by source order.

---

### 5. Gap Ledger

Documents the current state vs. what must change to support multi-clause function definitions.

#### 5.1 Parser (`src/compiler/parser.ark`)

| # | Current State | Required Change | Priority |
|---|---|---|---|
| G1 | `parse_fn_clause` exists and parses `fn name(pattern...) -> R = expr` | **Mostly complete**. Needs struct pattern support in `parse_pattern`. | High |
| G2 | `peek_is_fn_clause` exists for clause detection | **Complete**. No change needed. | None |
| G3 | `has_next_clause` exists for group collection | **Complete**. No change needed. | None |
| G4 | `desugar_multiclause` exists with desugaring logic | **Complete** for single-param case. Multi-param support is gated by error. | High |
| G5 | Multi-param clause support errors with "not yet supported" | **Remove the gate** and enable multi-param clause desugaring (code already exists in `desugar_multiclause` for the multi-param case at line 1768). | High |
| G6 | `parse_pattern` does not handle struct patterns (`Point { x, y }`) or char literals | **Add struct pattern parsing** and char literal pattern support. Struct patterns require `TK_LBRACE()` handling after an IDENT in pattern position. | High |
| G7 | Return type compatibility not checked across clauses | **Add cross-clause return type comparison** in the grouping logic (after line 1540). All non-empty return annotations must be structurally equal. | High |
| G8 | Type parameter propagation across clauses not implemented | **Propagate type params** from first clause to synthetic fn (similar to return type propagation). | Medium |
| G9 | No `pub` keyword propagation test for multi-clause groups | Verify `is_pub` flag propagates correctly (code at line 1550 passes `is_pub` to `desugar_multiclause` -- should work, add fixture). | Low |

#### 5.2 Lexer (`src/compiler/lexer.ark`, `src/compiler/lexer_kinds.ark`)

| # | Current State | Required Change | Priority |
|---|---|---|---|
| G10 | All required tokens exist: `TK_FN`, `TK_IDENT`, `TK_LPAREN`, `TK_RPAREN`, `TK_EQ`, `TK_ARROW`, `TK_UNDERSCORE`, `TK_COMMA`, `TK_INT`, `TK_STRING`, `TK_BOOL`, `TK_CHAR`, `TK_COLONCOLON`, `TK_LBRACE`, `TK_RBRACE` | **Complete**. No new tokens needed for this issue. | None |

#### 5.3 AST Node Kinds (`src/compiler/parser_kinds.ark`)

| # | Current State | Required Change | Priority |
|---|---|---|---|
| G11 | `NK_PAT_STRUCT(74)` is defined but **never produced** by the parser | **Implement struct pattern parsing** in `parse_pattern` to produce `NK_PAT_STRUCT` nodes. | High |
| G12 | `NK_PAT_TUPLE(75)` is defined and used in `desugar_multiclause` for multi-param | **Complete**. No change needed. | None |
| G13 | `NK_PAT_OR(76)` is defined and produced during match arm or-pattern parsing | **Complete**. Or-patterns in individual clause params are handled via existing `parse_pattern` → `parse_match_expr` or-pattern path. | None |

#### 5.4 Resolver (`src/compiler/resolver.ark`)

| # | Current State | Required Change | Priority |
|---|---|---|---|
| G14 | Resolver processes `NK_FN_DECL` nodes normally | **No change expected**. Desugaring happens before resolver sees the AST. | None (verify only) |

#### 5.5 Typechecker (`src/compiler/typechecker.ark`)

| # | Current State | Required Change | Priority |
|---|---|---|---|
| G15 | Typechecker processes `NK_FN_DECL` nodes normally | **No change expected**. Desugared function has normal params and a normal match body. | None (verify only) |

#### 5.6 CoreHIR / MIR Lowering (`src/compiler/corehir.ark`, `src/compiler/mir.ark`)

| # | Current State | Required Change | Priority |
|---|---|---|---|
| G16 | Downstream passes handle `NK_FN_DECL` with block body | **No change expected**. Desugared output is a normal fn with block body containing match expression. | None (verify only) |

#### 5.7 Fixtures (`tests/fixtures/selfhost/`)

| # | Current State | Required Change | Priority |
|---|---|---|---|
| G17 | 4 positive fixtures exist: `fn_multiclause_literal`, `fn_multiclause_ident`, `fn_multiclause_wildcard`, `fn_multiclause_single` | **Add** enum variant clause fixture (P4), struct pattern clause fixture (P5), string literal clause fixture (P7), pub visibility fixture (P8). | High |
| G18 | 2 negative fixtures exist: `fn_multiclause_mixed_arity`, `fn_multiclause_multi_param` | **Add** incompatible return annotation fixture (N3), mixed explicit/implicit return fixture (N4). | High |
| G19 | No `.expected` files for some clause fixtures | Verify each positive fixture has a corresponding `.expected` file. | Medium |

#### 5.8 Docs (`docs/language/spec.md`, `docs/language/syntax.md`)

| # | Current State | Required Change | Priority |
|---|---|---|---|
| G20 | `docs/language/spec.md` 6.1 describes only single-body `fn` | **Add** a subsection "6.1.1 Multi-Clause Function Definitions" with grammar and examples. Mark as `provisional`. | High |
| G21 | `docs/language/spec.md` Appendix A grammar has no `fn_clause_group` or `fn_clause` productions | **Add** `fn_clause_group` and `fn_clause` to `item` and as new productions. | High |
| G22 | `docs/language/syntax.md` does not mention clause syntax | **Add** a provisional section with the `fn ... = expr` form. | Medium |
| G23 | `docs/language/spec.md` 5.0 states patterns appear in `match` arms, `let` bindings, and `for` targets -- does not mention fn clause heads | **Update** to include `fn` clause heads as a pattern position. | Medium |
| G24 | `docs/data/language-doc-classifications.toml` needs new entry for clause syntax | **Add** classification entry for multi-clause fn definitions. | Low |

#### 5.9 Test Infrastructure

| # | Current State | Required Change | Priority |
|---|---|---|---|
| G25 | Fixture harness supports `.diag` files for expected diagnostic patterns | **Complete**. Negative fixtures can use existing `.diag` regex matching. | None |
| G26 | `scripts/manager.py verify selfhost-parity` validates diagnostic parity | **Complete**. Multi-clause negative fixtures will be validated through this path. | None |

---

### 6. Acceptance Criteria (Updated)

1. [ ] **Grammar implemented**: Parser accepts repeated `fn <name>(pattern...) -> R = expr` clauses as one logical function group.
2. [ ] **Single-body `fn` preserved**: All existing block-body `fn` definitions continue to parse and compile without change.
3. [ ] **Literal pattern clauses compile and run**: `fn_multiclause_literal.ark` passes.
4. [ ] **Identifier/wildcard clauses compile and run**: `fn_multiclause_wildcard.ark` and `fn_multiclause_ident.ark` pass.
5. [ ] **Enum variant clauses compile and run**: `fn_multiclause_enum_variant.ark` (P4) passes.
6. [ ] **Struct pattern clauses compile and run**: `fn_multiclause_struct_pattern.ark` (P5) passes.
7. [ ] **String literal clauses compile and run**: `fn_multiclause_string_literal.ark` (P7) passes.
8. [ ] **Mixed arity produces diagnostic**: `fn_multiclause_mixed_arity.ark` produces expected error.
9. [ ] **Incompatible return annotations produce diagnostic**: `fn_multiclause_incompatible_return.ark` (N3) produces expected error.
10. [ ] **Mixed explicit/implicit return produces diagnostic**: `fn_multiclause_mixed_return.ark` (N4) produces expected error.
11. [ ] **Multi-param support enabled**: `fn_multiclause_multi_param.ark` converted from negative to positive fixture (gate removed).
12. [ ] **Pub visibility propagates**: `fn_multiclause_pub.ark` (P8) compiles.
13. [ ] **Spec updated**: `docs/language/spec.md` has provisional section for multi-clause definition syntax.
14. [ ] **Grammar appendix updated**: EBNF in Appendix A includes `fn_clause_group` and `fn_clause` productions.
15. [ ] **Existing fixtures do not regress**: `python scripts/manager.py verify quick` passes.
16. [ ] **Selfhost parity**: `python scripts/manager.py selfhost fixture-parity` passes.

### 7. Implementation Order (Recommended)

| Step | Description | Gaps Addressed |
|---|---|---|
| 1 | Add struct pattern parsing to `parse_pattern` (handle `IDENTS pattern_args?` vs `IDENT "{" pattern_fields "}"`) | G6, G11 |
| 2 | Add char literal pattern to `parse_pattern` | G6 |
| 3 | Remove multi-param gate and enable multi-param desugaring (code already exists) | G5 |
| 4 | Add return type compatibility check across clauses in grouping logic | G7 |
| 5 | Add type parameter propagation from first clause to synthetic fn | G8 |
| 6 | Create positive fixtures (P4, P5, P7, P8) | G17 |
| 7 | Create negative fixtures (N3, N4) | G18 |
| 8 | Update `docs/language/spec.md` and `docs/language/syntax.md` | G20, G21, G22, G23 |
| 9 | Update `docs/data/language-doc-classifications.toml` | G24 |
| 10 | Verify no regressions, run full verification | G19, G25, G26 |

---

## Close gate

Close when: positive and negative fixtures for clause-based functions pass, existing fixtures
do not regress, and `docs/language/syntax.md` has a provisional section for the new syntax.
