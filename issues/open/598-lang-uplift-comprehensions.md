---
Status: open
Created: 2026-04-22
Updated: 2026-05-16
ID: 598
Track: selfhost-frontend / language-design
Orchestration class: design-ready
Depends on: —
Parent: 588
Initial restriction: array/Vec-style construction only. One generator, one optional filter.
In scope: Grammar, desugaring, type inference, positive/negative fixtures
Out of scope: Nested generators, dict comprehensions, lazy comprehensions, new runtime types
---

# Language Surface Uplift: Expression-Level Comprehensions

---

## Summary

Child issue for #588 Phase 4 — Expression-Level Comprehensions.

Add a lightweight collection-construction form that complements statement-style `for`.

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
- `src/compiler/mir_lower.ark`
- `src/compiler/lexer.ark`
- `src/compiler/lexer_kinds.ark`
- `src/compiler/parser_kinds.ark`
- `src/compiler/typechecker_kinds.ark`
- `src/compiler/resolver_kinds.ark`
- `src/compiler/mir_type_info.ark`
- `tests/fixtures/selfhost/`
- `tests/fixtures/manifest.txt`

## Allowed adjacent paths

- `docs/language/syntax.md`
- `docs/language/spec.md`

---

## Upstream / Depends on

None directly — can proceed independently.

## Blocks

- #599 (docs rollout covers all four features)

---

## STOP_IF

- Do not implement nested generators in the initial slice
- Do not add new collection runtime types to support comprehensions
- Do not change semantics of existing `for` statement

---

## Design Spec

### 1. Grammar Specification

#### 1.1 Syntax

The canonical form is:

```
'[' expr 'for' IDENT 'in' expr ('if' expr)? ']'
```

In EBNF (to be added to Appendix A of `docs/language/spec.md`):

```ebnf
(* Extension to primary_expr *)
primary_expr = "..."
             | "[" comprehension "]"
             | "...";

(* New productions *)
comprehension = expr "for" IDENT "in" expr ("if" expr)? ;
```

The `comprehension` production is added as an alternative case inside the existing `array_init` production or directly within `primary_expr` after the array literal alternatives. Specifically, the existing rule:

```ebnf
primary_expr = "..." | "[" array_init "]" | "...";
```

becomes:

```ebnf
primary_expr = "..." | "[" (array_init | comprehension) "]" | "...";
```

#### 1.2 Lexer Changes

No new keywords or tokens are required. The existing tokens are sufficient:

| Token | Use |
|-------|-----|
| `TK_LBRACKET` (`[`) | Opens comprehension |
| `TK_RBRACKET` (`]`) | Closes comprehension |
| `TK_FOR` (`for`) | Introduces generator clause |
| `TK_IDENT` | Loop variable name |
| `TK_IN` (`in`) | Separates variable from iterable |
| `TK_IF` (`if`) | Introduces optional filter |

The parser disambiguates: when `[` is followed by an expression and then `for`, it is a comprehension rather than an array literal. This is already implemented in `parse_primary` at line ~1205 of `src/compiler/parser.ark`.

#### 1.3 AST Node

The parser produces a single node kind:

```
NK_COMPREHENSION
  .text       = loop variable name (IDENT text)
  .children[0] = element expression (the expr before `for`)
  .children[1] = iter expression (the expr after `in`)
  .children[2] = filter expression (the expr after `if`; optional)
```

When no filter is present, `children` has length 2.

---

### 2. Desugaring Specification

The comprehension `[ELEM for VAR in ITER if FILTER]` desugars to the following
explicit for-loop + Vec builder pattern:

```ark
{
    let result: Vec<ElemType> = Vec_new_ElemType()
    for VAR in values(ITER) {
        if FILTER {
            push(result, ELEM)
        }
    }
    result
}
```

When no filter is present:

```ark
{
    let result: Vec<ElemType> = Vec_new_ElemType()
    for VAR in values(ITER) {
        push(result, ELEM)
    }
    result
}
```

#### 2.1 Desugaring invariants

| Property | Rule |
|----------|------|
| Result type | `Vec<E>` where `E` is the type of `ELEM` in a scope where `VAR: T` and `T` is the element type of `ITER` |
| Iterator protocol | The iter expression `ITER` is evaluated once and consumed via `values(ITER)` — the same `for x in values(v)` form from statement-level `for` |
| Builder construct | `Vec_new_*` + `push` — no new runtime primitives |
| Variable binding | `VAR` is a new immutable local binding in the loop scope, exactly as in `for VAR in ...` |
| Filter | `FILTER` is evaluated each iteration; if it produces `true`, `ELEM` is evaluated and pushed; if `false`, the iteration continues |
| No eval reordering | `ITER` is evaluated exactly once (before any iteration). `ELEM` is evaluated zero or more times (once per iteration where filter passes). `FILTER` is evaluated once per iteration |
| Tail expression | The comprehension is a block with a tail expression — the `result` variable is the block value |
| Type of result | The result local is of type `Vec<E>` where `E` is the inferred element type |
| `break` / `continue` | `break` and `continue` inside a comprehension are not allowed (they would refer to the implicit loop, which is confusing). The parser should reject them or they resolve to an outer loop if syntactically nested inside one |

#### 2.2 Existing `for` statement is unchanged

The statement-level `for` (parsed by `parse_for` at line 1489 of `parser.ark`) remains untouched. Comprehension parsing is independent, sharing only the `for` / `in` keyword tokens.

---

### 3. Type Inference Rules

#### 3.1 Iterable source type check

Given `[ELEM for VAR in ITER ...]`:

1. Infer the type of `ITER` — call it `T_iter`.
2. `T_iter` MUST be `Vec<E>` for some element type `E`.
   - If `T_iter` is not a `Vec`, emit a type error.
   - The error code should indicate "non-iterable source" (type checker diagnostic).
3. The loop variable `VAR` is bound to type `E` (the element type parameter of `T_iter`).
   - This replaces the current hardcoded `TY_I32` in `typechecker.ark` line 850.

#### 3.2 Element expression type check

1. Type-check `ELEM` in a scope where `VAR : E`.
   - The type of `ELEM` is the element type of the result — call it `T_elem`.

#### 3.3 Filter expression type check (optional)

1. If a filter `FILTER` is present:
   - Type-check `FILTER` in a scope where `VAR : E`.
   - `FILTER` MUST have type `bool`.
   - If the type is not `bool`, emit a type error.
   - Currently there is no such check in the typechecker; this must be added.

#### 3.4 Result type

1. The overall comprehension expression has type `Vec<T_elem>`.

#### 3.5 Formal rules

```
Γ ⊢ ITER : Vec<E>
Γ, VAR: E ⊢ ELEM : T
Γ, VAR: E ⊢ FILTER : bool   (if present)
─────────────────────────────────────────
Γ ⊢ [ELEM for VAR in ITER if FILTER] : Vec<T>
```

---

### 4. Positive Fixture Specifications

All positive fixtures are `run:` type (compile and execute, compare expected output).

#### 4.1 Fixture A: Map-only comprehension

**File:** `tests/fixtures/selfhost/comprehension_map.ark`
**Status:** EXISTS — no changes needed
**Manifest:** `run:selfhost/comprehension_map.ark`
**Purpose:** Verify `[expr for x in iter]` (no filter) produces correct values.

```ark
// Test: array comprehension — [expr for x in iter] (map-only)
use std::host::stdio

fn main() {
    let xs: Vec<i32> = Vec_new_i32()
    push(xs, 10)
    push(xs, 20)
    push(xs, 30)
    let doubled = [x * 2 for x in xs]
    let mut sum = 0
    for v in values(doubled) {
        sum = sum + v
    }
    stdio::println(i32_to_string(sum))
}
```

**Expected output:** `120` (i.e., 20 + 40 + 60)

#### 4.2 Fixture B: Map + filter comprehension

**File:** `tests/fixtures/selfhost/comprehension_map_filter.ark`
**Status:** EXISTS — no changes needed  
**Manifest:** `run:selfhost/comprehension_map_filter.ark`
**Purpose:** Verify `[expr for x in iter if cond]` filters correctly.

```ark
// Test: array comprehension — [expr for x in iter if filter] (map + filter)
use std::host::stdio

fn main() {
    let xs: Vec<i32> = Vec_new_i32()
    push(xs, 1)
    push(xs, 2)
    push(xs, 3)
    push(xs, 4)
    push(xs, 5)
    let evens_doubled = [x * 2 for x in xs if x % 2 == 0]
    let mut sum = 0
    for v in values(evens_doubled) {
        sum = sum + v
    }
    stdio::println(i32_to_string(sum))
}
```

**Expected output:** `12` (i.e., 4 + 8)

#### 4.3 Fixture C: Nested access in expression

**File:** `tests/fixtures/selfhost/comprehension_nested_access.ark`
**Status:** NEW — needs to be created
**Manifest:** `run:selfhost/comprehension_nested_access.ark`
**Purpose:** Verify the element expression supports field access (`x.field`) on the loop variable.

```ark
// Test: array comprehension — field access on loop variable in element expr
use std::host::stdio

struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let pts: Vec<Point> = Vec_new_Point()
    push(pts, Point { x: 1, y: 10 })
    push(pts, Point { x: 2, y: 20 })
    push(pts, Point { x: 3, y: 30 })
    let x_coords = [p.x for p in pts]
    let mut sum = 0
    for v in values(x_coords) {
        sum = sum + v
    }
    stdio::println(i32_to_string(sum))
}
```

**Expected output:** `6` (i.e., 1 + 2 + 3)

#### 4.4 Fixture D: Comprehension used as function argument

**File:** `tests/fixtures/selfhost/comprehension_as_arg.ark`
**Status:** NEW — needs to be created
**Manifest:** `run:selfhost/comprehension_as_arg.ark`
**Purpose:** Verify the comprehension expression can appear inline as a function argument (not only assigned to a variable).

```ark
// Test: array comprehension used inline as a function argument
use std::host::stdio

fn sum_vec(v: Vec<i32>) -> i32 {
    let mut s = 0
    for x in values(v) {
        s = s + x
    }
    s
}

fn main() {
    let xs: Vec<i32> = Vec_new_i32()
    push(xs, 1)
    push(xs, 2)
    push(xs, 3)
    let result = sum_vec([x * 10 for x in xs])
    stdio::println(i32_to_string(result))
}
```

**Expected output:** `60` (i.e., 10 + 20 + 30)

#### 4.5 Fixture E: Empty iterable comprehension

**File:** `tests/fixtures/selfhost/comprehension_empty_iter.ark`
**Status:** NEW — needs to be created
**Manifest:** `run:selfhost/comprehension_empty_iter.ark`
**Purpose:** Verify comprehension on an empty iterable produces an empty vector.

```ark
// Test: array comprehension on an empty iterable
use std::host::stdio

fn main() {
    let xs: Vec<i32> = Vec_new_i32()
    let result = [x * 2 for x in xs]
    stdio::println(i32_to_string(len(result)))
}
```

**Expected output:** `0`

---

### 5. Negative Fixture Specifications

Negative fixtures are `diag:` type (compiler must emit a diagnostic; exit code is checked).

#### 5.1 Fixture A: Non-iterable source

**File:** `tests/fixtures/selfhost/comprehen_deny_non_iterable.ark`
**Status:** NEW — needs to be created
**Manifest:** `diag:selfhost/comprehen_deny_non_iterable.ark`
**Purpose:** Verify that a non-`Vec` expression in the iterator position produces a type error.

```ark
// Error: non-iterable source in comprehension
fn main() {
    let x: i32 = 42
    let ys = [v * 2 for v in x]
}
```

**Expected diagnostic:**

```
error[E...|typecheck]:
```

(The specific error code will be assigned by the typechecker implementation.)

**Rationale:** Only `Vec<T>` values are iterable via `values()`. An `i32` is not iterable. The typechecker must reject the program.

#### 5.2 Fixture B: Filter expression not bool

**File:** `tests/fixtures/selfhost/comprehen_deny_filter_not_bool.ark`
**Status:** NEW — needs to be created
**Manifest:** `diag:selfhost/comprehen_deny_filter_not_bool.ark`
**Purpose:** Verify that a non-`bool` filter expression produces a type error.

```ark
// Error: filter expression must be bool
fn main() {
    let xs: Vec<i32> = Vec_new_i32()
    push(xs, 1)
    let ys = [x * 2 for x in xs if x]
}
```

**Expected diagnostic:**

```
error[E...|typecheck]:
```

(The specific error code will be assigned by the typechecker implementation.)

**Rationale:** The filter expression after `if` must evaluate to `bool`. An `i32` is not a `bool` and should not be coerced.

#### 5.3 Fixture C: Filter expression with unresolved name

**File:** `tests/fixtures/selfhost/comprehen_deny_unresolved_filter.ark`
**Status:** NEW — needs to be created
**Manifest:** `diag:selfhost/comprehen_deny_unresolved_filter.ark`
**Purpose:** Verify that an unresolved name in the filter expression is caught by the resolver.

```ark
// Error: unresolved name in filter expression
fn main() {
    let xs: Vec<i32> = Vec_new_i32()
    push(xs, 1)
    let ys = [x * 2 for x in xs if unknown_var > 0]
}
```

**Expected diagnostic:**

```
error[E...|resolve]:
```

#### 5.4 Fixture D: Wrong element type annotation on result

**File:** `tests/fixtures/selfhost/comprehen_deny_elem_type_mismatch.ark`
**Status:** NEW — needs to be created
**Manifest:** `diag:selfhost/comprehen_deny_elem_type_mismatch.ark`
**Purpose:** Verify that type annotation on the let-binding receiving a comprehension result is checked against the actual element type.

```ark
// Error: element type mismatch
fn main() {
    let xs: Vec<i32> = Vec_new_i32()
    push(xs, 1)
    let ys: Vec<String> = [x * 2 for x in xs]
}
```

**Expected diagnostic:**

```
error[E0200|typecheck]:
```

---

### 6. Implementation Notes

#### 6.1 Parser (`src/compiler/parser.ark`)

The parser at lines 1204-1231 already handles comprehension correctly. No changes required.

#### 6.2 Resolver (`src/compiler/resolver.ark`)

The resolver at lines 725-743 already handles `NK_COMPREHENSION` correctly. No changes required.

#### 6.3 Typechecker (`src/compiler/typechecker.ark`)

Current code at lines 841-859 has a partial implementation with issues:

1. **Line 850**: Loop variable is hardcoded to `TY_I32`. Must infer from iter expression.
   - After inferring `_iter_ty` (line 846), extract the element type from `Vec<E>`:
     - If `_iter_ty.tag == TY_VEC()`, the element type is `_iter_ty.type_args[0]`.
     - Otherwise, emit a "non-iterable source" error.
2. **Filter type check**: No filter type checking exists. Must add:
   - After type-checking `elem_ty`, if `children[2]` exists:
     - Infer `filter_ty = infer_expr(env, comp_scope, fn_sigs, children[2])`.
     - If `filter_ty.tag != TY_BOOL()`, emit a "filter not bool" error.

Corrected algorithm:

```
infer_expr(NK_COMPREHENSION):
  let iter_expr = children[1]
  let iter_ty = infer_expr(env, scope, fn_sigs, iter_expr)
  if iter_ty.tag != TY_VEC():
    emit error "non-iterable source"
    return TY_VEC() with TY_UNKNOWN() element
  let elem_ty_of_iter = iter_ty.type_args[0]   // the E in Vec<E>
  
  let comp_scope = child scope of current scope
  scope_define(comp_scope, node.text, elem_ty_of_iter)
  
  let elem_expr = children[0]
  let elem_result_ty = infer_expr(env, comp_scope, fn_sigs, elem_expr)
  
  if children[2] exists:
    let filter_expr = children[2]
    let filter_ty = infer_expr(env, comp_scope, fn_sigs, filter_expr)
    if filter_ty.tag != TY_BOOL():
      emit error "filter expression must be bool"
  
  return Vec<T> where T = elem_result_ty
```

#### 6.4 MIR Lowering (`src/compiler/mir_lower.ark`)

Current code at lines 3204-3343 hardcodes `i32` types. The generalized algorithm should:

1. Lower the iter expression to get the source Vec (already done).
2. Determine the element VT (value type) from the iterable's element type.
3. Allocate local for the loop variable with the correct VT.
4. Create the result Vec using the correct `Vec_new_*` factory based on element VT.
5. Loop with index, use `vec_get_unchecked` to fetch each element (already done).
6. Evaluate the filter expression (optional, already done).
7. Push the element expression result with `push(vec, elem)` (already done).
8. Return the result Vec.

The key generalization is moving from hardcoded `i32` types to using type information inferred by the typechecker and threaded through MIR.

#### 6.5 Lexer (`src/compiler/lexer.ark`)

No changes required. All needed tokens (`for`, `in`, `if`, `[`, `]`) are already tokenized.

---

### 7. Fixture Manifest Updates

The following entries must be added to `tests/fixtures/manifest.txt`:

```
run:selfhost/comprehension_map.ark           (exists)
run:selfhost/comprehension_map_filter.ark    (exists)
run:selfhost/comprehension_nested_access.ark (new)
run:selfhost/comprehension_as_arg.ark        (new)
run:selfhost/comprehension_empty_iter.ark    (new)
diag:selfhost/comprehen_deny_non_iterable.ark     (new)
diag:selfhost/comprehen_deny_filter_not_bool.ark  (new)
diag:selfhost/comprehen_deny_unresolved_filter.ark (new)
diag:selfhost/comprehen_deny_elem_type_mismatch.ark (new)
```

---

## Acceptance

- [ ] Parser: `[expr for x in iter]` and `[expr for x in iter if cond]` parse to `NK_COMPREHENSION` with correct children layout
- [ ] Resolver: Loop variable is defined in a child scope; element and filter expressions resolve against it
- [ ] Typechecker: Loop variable type is inferred from the iterable `Vec<E>`, not hardcoded
- [ ] Typechecker: Filter expression is checked to be `bool` when present
- [ ] Typechecker: Non-`Vec` iterable source is rejected with a diagnostic
- [ ] Typechecker: Result type is `Vec<T_elem>` where `T_elem` is the element expression type
- [ ] MIR lower: Comprehension lowers to Vec builder loop with correct type-specialized operations
- [ ] Positive fixture A (map-only) passes: `comprehension_map.ark` output matches expected
- [ ] Positive fixture B (map+filter) passes: `comprehension_map_filter.ark` output matches expected
- [ ] Positive fixture C (nested access) passes: `comprehension_nested_access.ark` output matches expected
- [ ] Positive fixture D (as function arg) passes: `comprehension_as_arg.ark` output matches expected
- [ ] Positive fixture E (empty iterable) passes: `comprehension_empty_iter.ark` output matches expected
- [ ] Negative fixture A (non-iterable source) produces a diagnostic: `comprehen_deny_non_iterable.ark`
- [ ] Negative fixture B (filter not bool) produces a diagnostic: `comprehen_deny_filter_not_bool.ark`
- [ ] Negative fixture C (unresolved in filter) produces a diagnostic: `comprehen_deny_unresolved_filter.ark`
- [ ] Negative fixture D (element type mismatch) produces a diagnostic: `comprehen_deny_elem_type_mismatch.ark`
- [ ] Existing `for` statement syntax and fixtures are unchanged
- [ ] All existing verification passes: `python scripts/manager.py verify quick && python scripts/manager.py verify fixtures`

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py verify fixtures
python scripts/manager.py selfhost fixture-parity
```

---

## Close gate

Close when:
1. The `[expr for x in iter if cond]` form parses, typechecks, and lowers correctly
2. All positive fixtures (map-only, map+filter, nested access, as-arg, empty-iter) produce correct output
3. All negative fixtures (non-iterable, filter-not-bool, unresolved-filter, elem-type-mismatch) produce expected diagnostics
4. Loop variable type is inferred from the iterable (not hardcoded `i32`)
5. Filter type is checked to be `bool`
6. Existing `for` statement fixtures still pass
7. The grammar in `docs/language/spec.md` Appendix A is updated with the comprehension production
8. `docs/language/spec.md` §3 gets a new subsection "3.20 Array Comprehension" documenting the feature
