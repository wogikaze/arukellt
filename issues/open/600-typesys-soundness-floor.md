---
Status: open
Created: 2026-04-22
Updated: 2026-05-16
ID: 600
Track: selfhost / typechecker
Orchestration class: design-ready
Depends on: —
Parent: 589
Close when: occurs check is implemented, function boundary enforcement is active,
  exhaustiveness covers Option/Result, all selfhost parity gates have no new FAILs.
---

Note: Phase 0 baseline record was completed by design spec agent on 2026-05-16.
See "## Phase 0 Baseline" and "## Design Spec" sections below.

# Type System Stage-Up: Soundness Floor

---

## Summary

Child issue for #589 Phase 1 -- Soundness Floor.

Before principled polymorphism can be layered onto the selfhost typechecker, the existing
inference engine must be made sound and explicit. This issue establishes the foundation:
occurs check, deep total substitution, function boundary contracts, and expanded exhaustiveness.

**Phase 0 baseline record is also part of this issue:** run all verify/parity commands,
record counts, write the gap ledger described in #589 Phase 0 before starting implementation.

---

## Phase 0 Baseline Record

Recorded: 2026-05-16

### Verification Baseline

| Command | Result | Count |
|---------|--------|-------|
| `python scripts/manager.py verify quick` | PASS | 22/22 checks passed (0 failed, 0 skipped) |
| `python scripts/manager.py selfhost diag-parity` | PASS | 1/1 checks passed |
| `python scripts/manager.py selfhost fixture-parity` | PASS | 1/1 checks passed |
| `python scripts/manager.py selfhost parity` | PASS | 1/1 checks passed |

All four gates pass cleanly at baseline. No regressions should be introduced by implementation.

### Feature-by-Feature Gap Ledger

#### 1. occurs_in_type check

**Current state: ALREADY IMPLEMENTED**

- `occurs_in_type(env, var_name, ty)` function exists at lines 226-247 of `src/compiler/typechecker.ark`
- Walks nested `type_args` recursively -- meets the "deep" requirement
- Called from `bind_var(env, var_name, ty)` at line 249-253 before binding
- Emits: `"occurs check failed: {var} appears in its own type"`
- Positive fixture exists: `tests/fixtures/selfhost/infer_occurs_check.ark` (proves normal inference works)

**Gaps:**
1. No negative fixture: there is no test case that actually triggers the occurs check error (e.g., `let x = x` or self-referential type via generic). The existing fixture only proves the positive case -- that normal inference still works.
2. No diagnostic code (E-prefix) assigned to the occur check error message.
3. Not documented in the language spec's Error Codes table.
4. No edge-case test for deeply nested self-referential types (e.g., `T -> Vec<T>` where `T` is being bound to something containing `T`).
5. The `occurs_in_type` function does not limit recursion depth -- pathological cases could overflow the stack.

#### 2. Deep/total substitution

**Current state: ALREADY IMPLEMENTED**

- `resolve_type_deep(env, ty)` at lines 290-304: recursively walks all `type_args` and resolves every nested `TY_TYPE_VAR`. Used at all specialization boundaries: monomorphization recording (line 937), method call recording (line 1007), and trait bound enforcement (line 622).
- `instantiate_type(ty, names, types)` at lines 634-658: replaces type parameters with concrete types in nested positions. Used in generic call instantiation (lines 927, 931) and method call instantiation (lines 990, 997, 1001).
- `mono_type_key(ty)` at lines 62-91: recursively generates distinct mangled keys for nested generic concretizations (e.g., `f<Vec<i32>>` vs `f<Vec<String>>`). Has a comment referencing `#312 slice-a`.
- `unify` at lines 197-212 resolves deeply before comparing type_args via `resolve_type_deep`.

**Gaps:**
1. No negative fixture proving that shallow-only substitution would be rejected (no regression guard).
2. `unify` at lines 176-224 does not use `resolve_type_deep` uniformly -- it only calls `resolve_type` (shallow) on type variables before comparison at lines 183-190. Deep resolution happens only in the "same tag" branch (line 197-198). This means if a type variable appears at the TOP level of `a` or `b` in `unify(a, b)`, only shallow resolution is used before binding. The deep resolution of type_args inside the resolved type occurs further down.
3. No spec documentation of the deep substitution contract.

#### 3. Function boundary contracts

**Current state: PARTIALLY IMPLEMENTED with behavioral gaps**

- `check_fn_body(env, fn_sigs, fn_node)` at lines 1160-1188 compares the final body expression type against the declared return type.
- The check (lines 1182-1186) uses `infer_expr` on the entire body node, then `unify` against `sig.return_type`.
- Negative fixture exists: `tests/fixtures/selfhost/ret_type_mismatch.ark` -- expects diagnostic `"type mismatch: String vs i32"`.
- Registered as `diag:selfhost/ret_type_mismatch.ark` in manifest.

**Gaps:**
1. The check SKIPS when `return_type` is `TY_UNKNOWN()` or `TY_UNIT()` (line 1183). Functions without explicit return type default to `()` and bypass the check entirely.
2. Explicit `return` statements inside the function body are NOT compared against the declared return type. The `check_stmt` for `NK_RETURN` (lines 1129-1135) merely infers the return value expression type and discards it. Only the FINAL block expression type is checked.
3. The diagnostic for a mismatch is the generic `"type mismatch: X vs Y"` with no indication that the mismatch is at a function boundary (no "return type mismatch" or "function 'bad' declared to return X but body returns Y").
4. No diagnostic code assigned.
5. No spec documentation of the function boundary contract.

#### 4. Exhaust pattern coverage

**Current state: PARTIALLY IMPLEMENTED**

- The match expression handler at lines 1021-1100 tracks patterns for: bool (true/false), Option (Some/None), Result (Ok/Err), and wildcard/identifier (catch-all).
- Exhaustiveness checks at lines 1082-1098 handle: `TY_BOOL` (must have true+false or wildcard), `TY_OPTION` (must have Some+None or wildcard), `TY_RESULT` (must have Ok+Err or wildcard).
- Positive fixture: `tests/fixtures/selfhost/typecheck_match_exhaustive.ark` (bool + int + wildcard). Registered as `run:`.
- Negative fixture: `tests/fixtures/selfhost/typecheck_match_nonexhaustive.ark` (Direction enum missing West). Registered as `diag:` but note: this diagnostic is produced by the Rust checker, NOT by the selfhost checker.

**Critical gap:**
1. User-defined enums (like `Direction { North, South, East, West }`) are NOT checked for exhaustiveness by the selfhost checker. The `resolved_scrutinee` for a user-defined enum resolves to `TY_ENUM` (tag 22), which is not handled by any of the three if-branches at lines 1083-1097.
2. No variant-name enumeration: the checker doesn't read the enum definition to know how many variants exist.
3. Nested pattern coverage is not checked (e.g., `Some(Some(x))` vs `Some(None)` vs `None` for `Option<Option<T>>`).
4. Struct patterns have no exhaustiveness check.
5. No diagnostic code assigned for non-exhaustive match messages.
6. No spec documentation of exhaustiveness requirements.

### Summary of Gaps by Severity

| Severity | Gap | Location |
|----------|-----|----------|
| HIGH | User-defined enum exhaustiveness not checked | `infer_expr` NK_MATCH_EXPR branch |
| HIGH | Explicit `return` statements not checked against return type | `check_stmt` NK_RETURN branch |
| MEDIUM | Function boundary diagnostic lacks context (no function name) | `check_fn_body` |
| MEDIUM | No negative fixture for occurs check | Fixtures |
| LOW | No diagnostic codes for occurs check, return mismatch, non-exhaustive | `typechecker.ark` |
| LOW | No spec documentation of soundness floor features | `docs/language/spec.md` |

### Existing Relevant Fixtures

| File | Type | What it tests |
|------|------|---------------|
| `infer_occurs_check.ark` | Positive (run) | Normal inference with occurs check installed |
| `ret_type_mismatch.ark` | Negative (diag) | Return type mismatch (Rust checker path) |
| `typecheck_match_exhaustive.ark` | Positive (run) | Bool + int + wildcard matches |
| `typecheck_match_nonexhaustive.ark` | Negative (diag) | Direction enum non-exhaustive (Rust checker only) |
| `typecheck_infer.ark` | Positive (run) | Basic type inference |
| `typecheck_infer_expr.ark` | Positive (run) | Expression-level inference |
| `typecheck_generic_call.ark` | Positive (run) | Generic function call with type inference |
| `typecheck_mono_instances.ark` | Positive (run) | Monomorphization instance tracking |
| `typecheck_trait_impl_smoke.ark` | Positive (run) | Trait impl + bound checking |

---

## Design Spec

### 1. occurs_in_type Check

#### Current code (no change needed -- already correct)

The `occurs_in_type` function at lines 226-247 is already correct:

```ark
fn occurs_in_type(env: TypeEnv, var_name: String, ty: TypeInfo) -> bool {
    if ty.tag == typechecker_kinds::TY_TYPE_VAR() {
        if eq(clone(ty.name), clone(var_name)) {
            return true
        }
        let resolved = resolve_type(env, ty)
        if resolved.tag == typechecker_kinds::TY_TYPE_VAR() && !eq(clone(resolved.name), clone(ty.name)) {
            return occurs_in_type(env, var_name, resolved)
        }
        return false
    }
    let arg_count = len(ty.type_args)
    let mut i = 0
    while i < arg_count {
        let arg = get_unchecked(ty.type_args, i)
        if occurs_in_type(env, var_name, arg) {
            return true
        }
        i = i + 1
    }
    false
}
```

This already:
- Checks if `var_name` directly matches the type variable name
- Follows type variable bindings via `resolve_type` to catch indirect self-references
- Walks nested `type_args` recursively

#### Required additions

1. **Negative fixture:** Add `infer_occurs_check_negative.ark` (or add to existing fixture) that triggers the occurs check error. Example:

   ```ark
   // This should fail with "occurs check failed: ..."
   fn self_ref() {
       let x = x  // self-referential: x appears in its own type
   }
   ```

   Also test nested generic self-reference: a function that would unify a type variable with a `Vec<T>` that contains itself.

2. **Diagnostic code:** Assign code `E0210` to the occurs check error message. Update `docs/language/spec.md` Error Codes table.

3. **Recursion guard (optional but recommended):** Add a recursion depth limit (e.g., 100 levels) to `occurs_in_type` to prevent stack overflow on pathological input.

4. **Spec documentation:** Add a subsection to `docs/language/spec.md` Section 2.5 (Type Inference) documenting that self-referential types are rejected.

#### Behavior specification

```
Given: unify(env, t, ty) where t is a type variable
When: t appears anywhere inside ty (including nested type_args of ty)
Then: bind_var MUST reject the binding and emit diagnostic:
      "occurs check failed: {t} appears in its own type"
```

```
Given: unify(env, ty, t) where t is a type variable
When: t appears anywhere inside ty
Then: Same rejection as above.
```

```
Given: occurs_in_type(env, var_name, ty)
When: ty references a type variable that resolves to another type variable
Then: The resolution chain MUST be followed (indirect occurrence detection).
```

### 2. Deep/Total Substitution

#### Current code (no change needed -- already correct)

Both `resolve_type_deep` and `instantiate_type` correctly walk nested `type_args`:

- `resolve_type_deep`: Calls `resolve_type` on head, then recurses on each type_arg.
- `instantiate_type`: Replaces type parameter variables in nested positions, recurses on type_args.
- `mono_type_key`: Recurses on type_args for distinct mangling.
- `unify`: Resolves deeply via `resolve_type_deep` before comparing type_args.

#### Required additions

1. **Regression fixture:** Add a positive fixture that exercises deeply nested generic instantiation to prove substitution is total. Example:

   ```ark
   fn wrap<T>(x: T) -> Vec<Vec<T>> {
       Vec_new_Vec_T(...)  // nested generic construction
   }
   ```

   This guards against shallow-substitution regressions.

2. **Spec documentation:** Document the deep substitution contract in `docs/language/spec.md` Section 2.7 (Generics): "Type parameters are substituted deeply at all levels of nested type arguments, including within Vec<T>, Option<T>, Result<T, E>, and user-defined generic types."

3. **Uniform resolution in unify:** Audit `unify` to ensure `resolve_type` (shallow) is used consistently at the top level and `resolve_type_deep` is used at the type_args level. This is already the case for the "same tag" branch. The type-var branches (lines 183-190) only need shallow resolution because binding a variable that resolves to a struct/enum will later trigger the "same tag" branch during re-unification.

#### Behavior specification

```
Given: instantiate_type(ty, [T1..Tn], [t1..tn])
When: ty contains T_i at any nesting depth within type_args
Then: All occurrences of T_i are replaced, recursively, depth-first.
```

```
Given: resolve_type_deep(env, ty)
When: ty or any of its nested type_args contain type variables
Then: All type variables are resolved transitively, depth-first.
```

```
Given: unify(env, a, b)
When: a and b have the same tag and both have type_args
Then: Each pair of corresponding type_args MUST be unified after deep resolution.
```

### 3. Function Boundary Contracts

#### Current code (needs behavioral fixes)

The `check_fn_body` function at lines 1160-1188 currently:
1. Calls `check_stmt(env, scope, fn_sigs, body)` to walk the function body
2. Looks up the function's declared return type from its `FnSig`
3. If return type is not UNKNOWN or UNIT: calls `infer_expr` again on the body and unifies with declared return type

#### Gaps in current implementation

1. **Explicit `return` statements are not checked.** `check_stmt` for `NK_RETURN` (lines 1129-1135) infers the return value expression but does NOT compare it against the declared return type. A function like:

   ```ark
   fn bad() -> i32 {
       return String_from("hello")  // should error, but doesn't
       String_from("world")         // last expression type
   }
   ```

   would only check the final expression `String_from("world")`, not the `return` statement.

2. **The diagnostic is generic.** A mismatch produces `"type mismatch: X vs Y"` with no indication that the mismatch is at a function boundary. It should say something like `"return type mismatch: function 'bad' declared to return i32 but body returns String"`.

3. **Unit return bypass.** Functions without explicit return type default to `()` (line 542 in `fn_sig_from_fn_decl`), and the check at line 1183 skips `TY_UNIT()`. This means `fn foo() { bar() }` (where `bar()` returns something non-unit) would not be checked. This is arguably correct behavior for void-returning functions (the value is discarded), but should be documented.

#### Design for the fix

**Target location:** `src/compiler/typechecker.ark`, `check_fn_body` function (line 1160) and `check_stmt` NK_RETURN handling (line 1129).

**Changes:**

A. In `check_fn_body`, improve the diagnostic message:

```ark
// Replace the generic unify call with an explicit comparison
let body_ty = infer_expr(env, scope, fn_sigs, body)
let resolved_ret = resolve_type(env, sig.return_type)
let resolved_body = resolve_type(env, body_ty)
if resolved_ret.tag != TY_UNKNOWN() && resolved_body.tag != TY_UNKNOWN() {
    if !types_are_compatible(resolved_ret, resolved_body) {
        let decl_name = type_display_name(sig.return_type)
        let body_name = type_display_name(body_ty)
        type_error(env, concat("return type mismatch: '", clone(fn_node.text),
                               "' declared to return ", decl_name,
                               " but body returns ", body_name))
    }
}
// Fall through to unify for side effects (type_args unification, etc.)
unify(env, body_ty, sig.return_type)
```

B. In `check_stmt` for `NK_RETURN`, add return type comparison:

```ark
if node.kind == typechecker_kinds::NK_RETURN() {
    if len(node.children) > 0 {
        let child = get_unchecked(node.children, 0)
        let ret_val_ty = infer_expr(env, scope, fn_sigs, child)
        // Compare against declared return type
        // (requires scope to carry a reference to the enclosing function's return type,
        //  or the check_fn_body can scan for return statements in the body after inference)
    }
    return
}
```

**Required infrastructure:** The simplest approach for `return` checking is to post-process: after `check_stmt(body)` completes, scan the body AST for `NK_RETURN` nodes and compare each return value expression type against `sig.return_type`. This avoids threading the return type through `check_stmt`'s signature.

#### Behavior specification

```
Given: fn f() -> T { <body> }
When: <body>'s final expression type is not compatible with T
Then: Diagnostic emitted: "return type mismatch: 'f' declared to return T but body returns <actual>"
      The diagnostic MUST include the function name.
```

```
Given: fn f() -> T { ...; return e; ... }
When: The type of e is not compatible with T
Then: Diagnostic emitted (same format as above).
```

```
Given: fn f() { <body> }
When: No return type is declared (defaults to ())
Then: No function boundary check is performed (body expression may be any type).
      This is the "command" vs "expression" function distinction.
```

```
Given: fn f() -> T { return e }
When: The final expression after the return statement is unreachable
Then: Only the return statement's value e is checked against T.
      (The final expression is dead code and should not produce a type mismatch error.)
```

### 4. Exhaust Pattern Coverage Expansion

#### Current code (needs expansion)

The match expression handler at lines 1021-1100 tracks:
- `has_wildcard`: set by NK_PAT_WILDCARD or NK_PAT_IDENT
- `has_bool_true` / `has_bool_false`: set by NK_PAT_LITERAL with text "true"/"false"
- `has_option_some` / `has_option_none`: set by NK_PAT_VARIANT with text matching Some/None
- `has_result_ok` / `has_result_err`: set by NK_PAT_VARIANT with text matching Ok/Err

Exhaustiveness checks (lines 1082-1098) handle only TY_BOOL, TY_OPTION, TY_RESULT.

#### Critical gap: user-defined enums

The selfhost checker does NOT check exhaustiveness for user-defined enums (tag `TY_ENUM`). The fixture `typecheck_match_nonexhaustive.ark` uses a `Direction` enum with 4 variants (North, South, East, West) and matches only 3. The diagnostic for this test is produced by the Rust checker only.

**Diagnostic mismatch documented:** The `.diag` file exists but no `.selfhost.diag` file exists for this fixture. The `diag:` manifest entry checks the Rust checker output only for this fixture.

#### Design for the fix

**Target location:** `src/compiler/typechecker.ark`, `infer_expr` function, NK_MATCH_EXPR branch (lines 1021-1100).

**Changes needed:**

A. **Resolve enum definition:** When the resolved scrutinee is `TY_ENUM`, look up the enum definition from `CheckCtx.enum_defs` (which is already populated in the struct definition, but needs to be passed into `infer_expr`). The `EnumInfo` struct (line 683) already has `variant_names: Vec<String>`.

B. **Track enum variant patterns:** For each match arm with `NK_PAT_VARIANT`, record the variant name. Compare against the known variant list from the enum definition.

C. **Emit diagnostic:** If any variant is uncovered and there's no wildcard, emit `"non-exhaustive match: missing <variant>"`.

**Infrastructure note:** This requires either:
- Threading `CheckCtx` (which has `enum_defs`) into `infer_expr`, OR
- Passing `enum_defs` as an additional parameter to `infer_expr`

Currently `infer_expr` takes `(env, scope, fn_sigs, node)` -- adding `enum_defs` (or the whole `CheckCtx`) is the cleanest approach.

#### Behavior specification

```
Given: match scrutinee { pat1 => e1, pat2 => e2, ... }
When: scrutinee resolves to TY_ENUM with known variants V1, V2, ..., Vn
Then: The checker MUST verify that every variant V_i appears in at least one arm pattern,
      OR there is a wildcard/identifier pattern that covers all remaining variants.
      If any variant is uncovered, emit:
      "non-exhaustive match: missing <variant>"
```

```
Given: match scrutinee { Some(x) => e1, None => e2 }
When: scrutinee resolves to TY_OPTION
Then: Accepted (both Some and None covered, as currently implemented).
```

```
Given: match scrutinee { Ok(x) => e1, Err(e) => e2 }
When: scrutinee resolves to TY_RESULT
Then: Accepted (both Ok and Err covered, as currently implemented).
```

```
Given: match scrutinee { true => e1, false => e2 }
When: scrutinee resolves to TY_BOOL
Then: Accepted (both true and false covered, as currently implemented).
```

```
Given: match scrutinee { _ => e1 }
When: scrutinee is any type
Then: Accepted (wildcard covers everything).
```

**Out of scope for this issue:**
- Nested pattern coverage (e.g., `Some(Some(x))` vs `Some(None)` vs `None`)
- Guard expression coverage analysis
- Refutable let patterns
- Witness construction (showing a counterexample value)

---

## Fixture Specifications

### Positive fixtures (should compile and run)

| File | Purpose | Key test |
|------|---------|----------|
| `infer_occurs_check.ark` | Already exists | Normal inference with occurs check installed |
| `typecheck_match_exhaustive.ark` | Already exists | Bool + int + wildcard matching |
| Existing typecheck_infer/infer_expr/generic_call/mono_instances | Already exist | Various positive cases |

### Negative fixtures (should produce diagnostic)

| File | Purpose | Expected diagnostic |
|------|---------|---------------------|
| `infer_occurs_check_negative.ark` (NEW) | Self-referential type rejection | `"occurs check failed: tX appears in its own type"` |
| `ret_type_mismatch.ark` | Already exists | `"type mismatch: String vs i32"` (improve message as part of fix) |
| `ret_stmt_mismatch.ark` (NEW) | Return statement inside body mismatches return type | `"return type mismatch: 'bad' declared to return i32 but return value is String"` |
| `match_non_exhaustive_option.ark` (NEW) | Option match missing Some arm | `"non-exhaustive match: Option requires 'Some' and 'None' arms or a wildcard"` |
| `match_non_exhaustive_result.ark` (NEW) | Result match missing Err arm | `"non-exhaustive match: Result requires 'Ok' and 'Err' arms or a wildcard"` |
| `match_non_exhaustive_enum.ark` (NEW) | User-defined enum missing variant | `"non-exhaustive match: missing Direction::West"` |

### Fixture patterns

Each negative fixture should follow the existing pattern:
1. `.ark` file with the source code and a comment describing the expected failure
2. `.selfhost.diag` file with the expected diagnostic text (one line per diagnostic)
3. Entry in `tests/fixtures/manifest.txt` as `diag:selfhost/<name>.ark`

---

## Dependencies and Ordering

### Implementation order

The four features are largely independent but have a suggested ordering:

1. **occurs_in_type negative fixtures FIRST** (lowest risk, no code change needed)
   - Add `tests/fixtures/selfhost/infer_occurs_check_negative.ark`
   - Add `tests/fixtures/selfhost/infer_occurs_check_negative.selfhost.diag`
   - Register in manifest
   - Verify: `python scripts/manager.py selfhost diag-parity`

2. **Function boundary contracts SECOND** (medium risk, code change in typechecker.ark)
   - Improve `check_fn_body` diagnostic message
   - Add return-statement checking (post-processing scan of body AST)
   - Add `ret_stmt_mismatch.ark` + `.selfhost.diag`
   - Verify: old test still passes, new test produces diagnostic

3. **Exhaust pattern coverage for user-defined enums THIRD** (medium risk, code change)
   - Thread `enum_defs` into `infer_expr`
   - Add TY_ENUM exhaustiveness check in NK_MATCH_EXPR branch
   - Add `match_non_exhaustive_enum.ark` + `.selfhost.diag`
   - Verify: existing Option/Result/bool checks still work, new enum check works

4. **Deep substitution regression guard LAST** (lowest risk, fixture only)
   - Add nested generic instantiation positive fixture
   - Verify: all tests pass

### Test command sequence

After each implementation step:

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

After all steps complete:

```bash
python scripts/manager.py selfhost parity
python scripts/manager.py verify quick
```

### Blocking relationships

- Does NOT block on #312 or #495 (they are upstream blockers for later phases)
- Is blocked BY nothing
- BLOCKS #601 (TypeScheme/generalization)
- BLOCKS #312 partial closure (deep substitution gap partially closes #312)

---

## Scope

**In scope:**
- Phase 0: Record baseline (observe only, no implementation)
- `occurs_in_type` -- reject bind_var when var occurs inside the target type
- Deep / total substitution for nested type arguments at all comparison and specialization boundaries
- Function boundary contracts: compare return expressions and final body type against declared return type
- Exhaust pattern coverage beyond bool-only toward current enums / Option / Result

**Out of scope:**
- TypeScheme / generalization -- that is #601
- Obligation-based trait solving -- that is #602
- Monomorphization contract -- that is #603
- Any new surface syntax changes
- Nested pattern coverage analysis (e.g., `Some(Some(x))`)
- Witness construction for non-exhaustive matches

---

## Primary paths

- `src/compiler/typechecker.ark`
- `src/compiler/typechecker_kinds.ark`
- `tests/fixtures/selfhost/` (new negative fixtures for occurs check, function boundary, exhaustiveness)
- `tests/fixtures/manifest.txt` (register new fixtures)
- `docs/language/spec.md` (Error Codes table update)

## Allowed adjacent paths

- `docs/language/spec.md` (update Error Codes table, add soundness documentation)

---

## Upstream / Depends on

None.
Note: #312 (generic monomorphization) and #495 (trait bounds) are upstream blockers for
later phases, but do not block this issue.

## Blocks

- #601 (TypeScheme work requires soundness floor to be stable)
- #312 (monomorphization gap partially closeable after deep substitution is fixed)

---

## Acceptance

1. `occurs_in_type` has a negative fixture proving self-referential types are rejected
2. Function bodies whose final expression type mismatches the declared return type produce a diagnostic that identifies the function and both types
3. Explicit `return` statements inside a function body are checked against the declared return type
4. Selfhost exhaustiveness checker handles user-defined enums (variant enumeration) in addition to the existing bool/Option/Result coverage
5. All new fixtures have `.selfhost.diag` files and are registered in manifest.txt
6. All four selfhost parity gates have no new FAILs compared to Phase 0 baseline

---

## Required verification

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
python scripts/manager.py selfhost parity
```

---

## STOP_IF

- Do not start TypeScheme / generalization in this issue
- Do not start obligation-based solving in this issue
- Do not change surface syntax
- Do not attempt nested pattern coverage analysis

---

## Close gate

Close when: occurs check has negative fixture coverage, function boundary enforcement
produces descriptive diagnostics including for explicit `return` statements,
exhaustiveness covers user-defined enums (not just bool/Option/Result),
all selfhost parity gates have no new FAILs, and new fixtures are registered.
