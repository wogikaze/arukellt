---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
ID: 688
Track: language-design
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 688 — Trait method dispatch inside generic functions

## Summary

Arukellt already supports `trait` declarations, `impl Trait for Type` blocks,
and **trait bound checking** in generic function signatures
(`src/compiler/typechecker/trait_bounds.ark`, fixture
`tests/fixtures/generics_v1/trait_bound.ark`). However the bound fixture only
prints a constant string and **never calls `item.show()`** inside the generic
body. The emitter has no trait dispatch / vtable code
(`grep -rln trait src/compiler/emit_*.ark` returns nothing relevant).

This means: **a generic function `<T: Trait>` can declare and check the bound
but cannot actually invoke a trait method on `T`**. This is the root blocker
for every stdlib trait-based abstraction (`Iterator::next`, `Clone::clone`,
`Read::read`, `Display::to_string` dispatched generically, `Hash::hash` used by
`HashMap<K: Hash>`, etc.).

## Current state

- `trait Eq { fn eq(self: Eq, other: Eq) -> bool }` parses and typechecks.
- `impl Eq for i32 { ... }` registers in `env.trait_impls`.
- `fn f<T: Eq>(x: T)` — bound is enforced (`enforce_trait_bound`).
- `x.eq(y)` inside `f` — **not emitted / not supported**. No static
  specialization, no dynamic vtable dispatch.
- `std::core::cmp::Eq`, `std::core::convert::Display`, `std::core::hash::Hash`
  are defined but only callable concretely (e.g. `i32_to_string`), not through
  generic trait dispatch.

## Rust baseline

Rust resolves trait method calls either by **static dispatch** (monomorphization
+ specialization) or **dynamic dispatch** (`dyn Trait` vtable). For a stdlib
parity surface, monomorphization-based static dispatch is the minimum; `dyn
Trait` is a follow-up.

## Required work

- [x] MIR lowering: resolve `x.trait_method(...)` calls inside `<T: Trait>`
  bodies to the concrete method after monomorphization, OR introduce a vtable
  representation passed as an implicit parameter.
- [x] Emitter: emit the resolved method call (direct call for static dispatch,
  `call_indirect` for dynamic).
- [x] Typechecker: allow trait method resolution against the declared bound
  (currently only bound *checking* works, not method *lookup*).
- [x] Fixture: generic function that actually invokes a trait method on the
  bounded parameter and verifies output (e.g. `fn print<T: Display>(x: T)`
  calling `x.to_string()`).
- [x] Fixture: multiple impls of the same trait dispatched from one generic
  caller.
- [ ] Decide and document static-vs-dynamic dispatch strategy (ADR candidate).
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Implementation (2026-06-26)

**Approach: static dispatch via monomorphization.**

### Typechecker changes

- `trait_method_record.ark` / `contract_trait_method_record.ark`: new
  `TraitMethodEntry { trait_name, method_name, sig }` record.
- `trait_method_registry.ark`: collects method signatures from `trait`
  declarations into `env.trait_methods`; `lookup_trait_method_sig` retrieves
  a method `FnSig` by `(trait_name, method_name)`.
- `contract_env_record.ark`: `TypeEnv` gains `trait_methods`,
  `current_fn_type_params`, `current_fn_bounds` fields.
- `module.ark`: `register_trait_impl_decls` now also calls
  `register_trait_method_sigs`.
- `module_env_merge.ark`: `type_env_for_fn_check` propagates
  `trait_methods`, `current_fn_type_params`, `current_fn_bounds`.
- `body.ark`: `check_fn_body` sets `env.current_fn_bounds` and
  `env.current_fn_type_params` from the function's type parameter bounds.
- `call_method.ark`: when `lookup_fn_sig` fails, falls back to
  `try_resolve_trait_method_from_bounds` which checks if the receiver is a
  `TY_TYPE_VAR` and searches the current function's bounds for a trait that
  declares the called method.

### MIR lowering changes

- `ctx_types.ark` / `ctx_init.ark`: `LowerCtx` gains `mono_type_param_names`,
  `mono_type_param_types`, `mono_instances` fields.
- `ctx_mono_type_params.ark` (new): provides `ctx_resolve_mono_type_param`
  to map `?T` -> concrete type name, and `ctx_setup_mono_type_params_by_ordinal`
  which collects type variable names from the current function's locals and
  pairs them with the `MonoInstance`'s type arguments by ordinal.
- `entry_context.ark`: stores `mono_instances` list into `ctx` for later
  lookup.
- `entry_emit_top.ark` / `entry_emit_method.ark`: after binding parameters,
  calls `ctx_setup_mono_type_params_by_ordinal` with the emit (mangled)
  function name; clears after body emission.
- `method.ark`: `mir_initial_method_callee` now checks if the receiver's
  local type starts with `?` (type variable) and resolves it via
  `ctx_resolve_mono_type_param` before constructing `Type::method`.

### Fixture

- `tests/fixtures/generics_v1/trait_method_dispatch.ark`: defines `trait Greet`
  with `fn greet(self) -> String`, two impls (`Point`, `Vec2`), and a generic
  `fn print_greeting<T: Greet>(item: T)` that calls `item.greet()`.

## Acceptance

- [x] A generic function `<T: Trait>` can call a trait method on its parameter
      and the call resolves at compile time to the correct impl.
- [x] At least two distinct types implementing the same trait are dispatched
      correctly from one generic function in a fixture.
- [ ] Existing `Eq` / `Display` / `Hash` trait definitions in `std::core`
      become callable through generic dispatch (not only via concrete
      wrappers).
- [ ] Dispatch strategy documented (ADR or `docs/stdlib/` section).
- [ ] `python3 scripts/manager.py verify quick` exits 0.


## References

- `src/compiler/typechecker/traits.ark`
- `src/compiler/typechecker/trait_bounds.ark`
- `src/compiler/typechecker/trait_registry.ark`
- `src/compiler/hir/item_trait_record.ark`
- `tests/fixtures/generics_v1/trait_bound.ark` (bound only, no method call)
- `std/core/cmp.ark`, `std/core/convert.ark`, `std/core/hash.ark`
- Blocks: #691, #692, #693, #694, #695, #696
