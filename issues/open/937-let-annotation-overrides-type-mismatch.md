---
Status: open
Created: 2026-10-27
Updated: 2026-10-30
ID: 937
Track: compiler
Depends on: "#934"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#934"
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "#815, #933, #935, #938"
---

# 937 — `let` type annotations silently replace a mismatching inferred type

## Summary

When the inferred initializer type and the annotation share a constructor
(`Vec`, `Option`, enum, struct) but differ in a type argument,
`should_prefer_annotation` returns true and the binding takes the
annotation without unifying. A `Vec<i32>` can be bound as `Vec<String>`
with no diagnostic.

## Evidence (2026-10-27)

- `src/compiler/typechecker/stmt_let.ark` L43–58 (annotation override, with
  only comprehensions excluded since #815) and L208–237
  (`should_prefer_annotation`: returns true when any type-argument tag
  differs).

Repro (`ext_let.ark`):

```ark
use std::host::stdio

fn ints() -> Vec<i32> {
    let v = Vec::new<i32>()
    push(v, 7)
    v
}

fn main() {
    let v: Vec<String> = ints()
    stdio::println(get_unchecked(v, 0))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: compiles; `wasmtime run` prints an empty line (the `i32` 7 read
  as a string).
- `wasm32-gc`: "compiler produced invalid core Wasm".

## Expected

E0200 `Vec<i32>` vs `Vec<String>` at the `let`, on both targets.

## Acceptance

- [ ] `should_prefer_annotation` is removed; annotations are always unified
      with the inferred type through the single type-compatibility routine
      shared with #935 / #938.
      (Implemented and reverted before merge: it regresses 108 fixtures until
      #934 fixes weak inference. See "Lead integration decision" below.)
- [ ] The weak-inference cases the override was hiding (e.g. `from_iter`
      picking the first impl by name) are fixed at their source (#934), not
      re-allowlisted. (Not fixed: #934 is still open. The mismatches are now
      reported instead of hidden; see Verification in #938.)
- [ ] Compile-error fixture for the repro on `wasm32` and `wasm32-gc`.
      (Written as `tests/fixtures/diagnostics/let_annotation_type_arg_mismatch.ark`/`.diag`
      and verified with the reverted experiment, then removed together with it:
      on the current tree the repro compiles, so the fixture would not pass.
      Re-add with the #934-first re-apply.)
- [ ] `python3 scripts/manager.py selfhost fixpoint` stays green.
      (Not run — this lane was instructed not to run fixpoint.)

## Implementation (2026-10-30)

`src/compiler/typechecker/stmt_let.ark`: `should_prefer_annotation` is deleted,
together with the comprehension special case (#815) that only existed to
pierce the override. The annotation path is now one path for every
initializer:

1. `unify_compat::is_type_compatible(bind_ty, ann_ty)` — the shared recursive
   predicate from #938 — decides whether the inferred initializer type and the
   annotation are the same type, including type arguments;
2. when compatible, `unify::unify(env, ann_ty, bind_ty)` (annotation first, so
   an initializer naming a generic type without arguments such as
   `Vec::new()` takes the annotated arguments; unbound variables are bound);
3. when incompatible, E0200 `type mismatch: initializer <init> does not match
   annotation <ann>`, then the annotation type is bound so later uses do not
   cascade.

The weak-inference cases the override was hiding are surfaced, not
re-allowlisted: `Vec::from_iter(...)` and `pop(Vec<String>)` still resolve by
name to their first (`i32`) impl, so annotations that previously "won" now
report a mismatch in std code and fixtures. Fixing that inference is #934.

## Verification (2026-10-30)

Isolated build and full parity numbers are in #938 Verification (same compiler:
sha256 `4ea395da7bd6c7710955fc0ad3c3b44ed29414a00c24ca8c7ead55f598e293f5`,
`PASS=1811 FAIL=108`, `test: PASS=19 FAIL=19`). Repro `wt/lane30/p937.ark` now
gives `error[E0200|typecheck]: type mismatch: initializer Vec<i32> does not
match annotation Vec<String>` on both `wasm32` and `wasm32-gc`; the same
initializer bound as `Vec<String>` before this change.

Fixture: `tests/fixtures/diagnostics/let_annotation_type_arg_mismatch.ark` and
`.diag`, verified on both targets. `manifest.txt` was not edited (Lead
registers `diag:diagnostics/let_annotation_type_arg_mismatch.ark` and the
return-check twin).

## Lead integration decision (2026-10-30) — change reverted; issue stays blocked on #934

The implementation above was removed from the tree at integration, because it
cannot land before #934 without regressing the fixture suite:

- Full parity with the `stmt_let.ark` change: `PASS=1811 FAIL=108` (run) and
  `test: PASS=19 FAIL=19`. Every failure is the new E0200, and 95 + 3 of them
  come from weak inference in std (`pop` and `Vec::from_iter` resolving by
  name to the `i32` shape), i.e. #934.
- The 10 remaining failures are fixture code the old override hid
  (`let v: Vec<Vec<i32>> = Vec::new<i32>()`); one of them,
  `tests/fixtures/diagnostics/nested_generic.ark`, was corrected in place and
  is kept (it is the honestly typed initializer and compiles under both rules).
- The diagnostic fixture `diagnostics/let_annotation_type_arg_mismatch.ark`
  was therefore removed together with `src/compiler/typechecker/stmt_let.ark`'s
  change. `wt/lane30/p937.ark` compiles again on the final stage-2
  (`sha256 6c923e5d…`), which is the current, unfixed behaviour.

What did land from this lane is #938's shared predicate
(`typechecker/unify_compat.ark`) used by the return check. #937 keeps
`Depends on: #934` and `blocked-by-upstream`: fix weak inference there, then
re-apply the `should_prefer_annotation` removal and the two diagnostic
fixtures.


