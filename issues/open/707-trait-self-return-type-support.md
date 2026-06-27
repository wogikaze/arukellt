---
Status: open
Created: 2026-06-29
Updated: 2026-06-30
ID: 707
Track: language-design
Depends on: 688
Orchestration class: design-required
Orchestration upstream: None
Blocks v{N}: "689, 691, 692"
Priority: 1
Source: Issue #692 implementation — Clone trait generic dispatch blocked by Self return type
---

# 707 — `Self` return type support for trait method dispatch

## Summary

Arukellt's trait method dispatch (implemented in #688) resolves trait method
signatures from the trait registry and uses the signature's return type
directly. When a trait method returns the trait's own type (e.g.
`fn clone(self: Clone) -> Clone`), the typechecker resolves the return type
`Clone` as a `TY_STRUCT` named "Clone" — a phantom struct that does not
exist in the type environment. This produces invalid WASM (type mismatch:
expected `(ref null $type)`, found `i32`) because the return type should be
the receiver's concrete type (the `Self` substitution).

Rust solves this with the `Self` keyword: `fn clone(&self) -> Self`. Arukellt
has no `Self` keyword, and the typechecker has no mechanism to substitute the
receiver's type for the trait name in trait method return positions.

This blocks every trait that returns `Self`:
- `Clone::clone` → `Self` (worked around in #692 with `Clone<T>` type param)
- `Add::add` / `Sub::sub` / `Mul::mul` / ... → `Self` (#689 operator overload)
- `Iterator::next` → `Option<Self::Item>` (partially — #691)
- `Deref::deref` → `Self::Target`
- `AsRef::as_ref` → `Self::Target`

## Current state

- `trait Clone { fn clone(self: Clone) -> Clone }` — the typechecker resolves
  the return type `Clone` as `TY_STRUCT` with name "Clone".
- `infer_trait_method_call` in `src/compiler/typechecker/call_method.ark`
  returns `FnSig_return_type(sig)` directly — no `Self` substitution.
- Workaround in #692: `trait Clone<T> { fn clone(self: Clone<T>) -> T }`
  uses a type parameter instead of `Self`. This works for simple cases but
  is not idiomatic and does not generalize to operator traits
  (`Add::add` needs `Self`, not a type param).
- Existing traits that work (`Display::to_string` → `String`, `Eq::eq` →
  `bool`, `Hash::hash` → `i32`) all return concrete types, not `Self`.

## Rust baseline

Rust uses the `Self` keyword in trait definitions:

```rust
trait Clone {
    fn clone(&self) -> Self;
}

trait Add<Rhs = Self> {
    type Output;
    fn add(self, rhs: Rhs) -> Self::Output;
}
```

The compiler substitutes `Self` with the concrete implementing type during
monomorphization. Arukellt uses static dispatch (monomorphization) via
`call_generic_method.ark`, so the concrete type is always known at the
trait method call site.

## Proposed approach

Two options (design decision required):

### Option A: `Self` keyword (idiomatic, larger scope)

1. **Parser**: Add `TK_SELF` token for `Self` keyword.
2. **Typechecker**: In trait method signatures, `Self` resolves to a
   `TY_TYPE_VAR` bound to the trait's implementing type. During trait method
   dispatch (`infer_trait_method_call`), substitute `Self` with the
   receiver's concrete type.
3. **Trait definitions updated**: `fn clone(self: Self) -> Self`,
   `fn add(self: Self, rhs: Self) -> Self`, etc.
4. **MIR lowering**: `Self` type var is resolved during monomorphization
   (same as existing type param substitution).

### Option B: Trait-name-as-Self substitution (minimal, no new keyword)

1. **Typechecker**: In `infer_trait_method_call`
   (`src/compiler/typechecker/call_method.ark`), detect when the return type
   is a `TY_STRUCT` whose name matches the trait name and whose first
   parameter type also matches. Substitute the receiver's concrete type.
2. **No parser changes** — trait definitions keep
   `fn clone(self: Clone) -> Clone`.
3. **Limitation**: Only works for `Self` in return position and first-param
   position. Does not support `Self` in nested positions
   (e.g. `Option<Self>`, `Vec<Self>`).

### Recommendation

Option B is a minimal fix that unblocks #689 and simplifies #692's
`Clone<T>` workaround back to `Clone`. Option A is the proper long-term
solution but requires parser changes and is a larger scope.

**Suggested path**: Implement Option B first (unblocks #689, #691 partial),
then Option A as a follow-up when associated types (#701 extension) are
needed.

## Required work

### Option B (minimal)

- [ ] **Typechecker** (`src/compiler/typechecker/call_method.ark`):
      In `infer_trait_method_call`, after obtaining `ret_ty`, check if
      `ret_ty` is `TY_STRUCT` with a name matching the trait's first
      parameter type name. If so, substitute `recv_ty` (the receiver's
      inferred type) as the return type.
- [ ] **Typechecker**: Apply the same substitution to all parameter types
      that match the trait name (not just the return type), so that
      `fn add(self: Add, rhs: Add) -> Add` correctly constrains both
      `self` and `rhs` to the receiver's type.
- [ ] **MIR lowering**: Verify that the substituted return type propagates
      correctly through `call_generic_method.ark` monomorphization.
- [ ] **WASM emitter**: Verify that the concrete return type (not the
      phantom `TY_STRUCT`) reaches the WASM function signature.

### Option A (full `Self` keyword — follow-up)

- [x] **Parser**: Add `Self` keyword token and AST node.
      — `TK_SELF` (34) added to `tokens.ark`, `keywords_decl.ark`,
        `name_keywords_decl.ark`. `type_token_predicates::is_named_start`
        accepts `TK_SELF`. `parse_named_type` produces `NK_TYPE_NAMED("Self")`.
- [x] **Typechecker**: `Self` in trait method signatures resolves to a
      type variable bound to the impl's self type.
      — `resolve_type_name_generic` in `type_generic.ark` maps "Self" to
        `TY_TYPE_VAR("Self")`.
      — `infer_trait_method_call` in `call_method.ark` substitutes
        `TY_TYPE_VAR("Self")` return type with the receiver's concrete type.
      — `infer_method_call_expr` prefers trait dispatch over prelude
        free functions when the receiver is a `TY_TYPE_VAR` (#707).
      — `typed_fn_return_type` in `module_results.ark` uses
        `resolve_type_ann_node_generic` so generic return types resolve
        as `TY_TYPE_VAR` instead of `TY_STRUCT`.
- [x] **MIR lowering**: `Self` type var is resolved during monomorphization.
      — `ctx_apply_fn_return_type_for_rt_tag` in `ctx_locals_fn.ark` maps
        `TY_TYPE_VAR` (30) to `VT_I32` for generic stubs.
      — `entry_fns_mono.ark` substitutes `TY_TYPE_VAR` return type with
        the concrete type from `MonoInstance.type_args` for each variant.
- [x] **WASM emitter**: `val_type_to_sig` in `sections_type_plan.ark` maps
      `TY_TYPE_VAR` (30) to `"i32"` as a fallback for generic stubs.
- [x] **Trait definitions**: Migrated `Clone` to use `Self`.
      — `std/core/clone.ark`: `trait Clone { fn clone(self: Self) -> Self }`.
- [ ] **Fixtures**: Verify `Self` in nested positions
      (`Option<Self>`, `Vec<Self>`).

### Fixtures

- [x] `tests/fixtures/stdlib_trait/self_return_clone.ark` —
      `fn dup<T: Clone>(x: T) -> T { x.clone() }` works through generic
      dispatch with `trait Clone { fn clone(self: Self) -> Self }`.
      Compiles successfully; WASM validation blocked by pre-existing
      `read_to_string` codegen issue (func 10), not by Self type handling.
- [ ] `tests/fixtures/stdlib_trait/self_return_add.ark` —
      `fn add_any<T: Add>(x: T, y: T) -> T { x.add(y) }` works through
      generic dispatch.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `fn dup<T: Clone>(x: T) -> T { x.clone() }` compiles and runs correctly
      with `trait Clone { fn clone(self: Clone) -> Clone }` (no type param
      workaround).
- [ ] `fn add_any<T: Add>(x: T, y: T) -> T { x.add(y) }` compiles and runs
      correctly.
- [ ] Existing trait dispatch (`Display::to_string`, `Eq::eq`, `Hash::hash`)
      continues to work unchanged.
- [ ] #692's `Clone<T>` workaround can be simplified back to `Clone`.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait method dispatch — provides the dispatch
  infrastructure that this issue extends)
- Blocks: #689 (operator overload — `Add::add` returns `Self`),
  #691 (Iterator — `next` returns `Option<Self::Item>`),
  #692 (Clone — `Clone<T>` workaround can be removed once Self lands)
- Related: #701 (associated function syntax — `Self` keyword is a
  prerequisite for full associated type support)
- `src/compiler/typechecker/call_method.ark` — `infer_trait_method_call`
- `src/compiler/typechecker/trait_method_registry.ark` — trait sig registry
- `src/compiler/typechecker/call_generic_method.ark` — monomorphization
- `std/core/clone.ark` — current `Clone<T>` workaround
