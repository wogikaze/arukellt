---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 946
Track: typecheck
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-062, ADR-078, ADR-089, #945"
---

# 946 — single-uppercase type names in impl heads are treated as implicit type parameters

## Summary

Without an explicit binder, impl heads infer type parameters with a
"single-uppercase convention". A real struct named with one uppercase
letter is therefore read as a type variable. `impl Copy for S {}` for
`struct S` is rejected as a user blanket impl (E0216). Other one-letter
nominal types are affected in the same way. Name shape decides whether an
identifier is a type or a variable, which makes the grammar ambiguous.

## Evidence (2026-10-27)

- `src/compiler/typechecker/impl_generics.ark` L57–80 (`type_param_names`:
  implicit parameters from "the existing single-uppercase convention").
- `src/compiler/typechecker/trait_registry.ark` L199–210 (var-carrying
  self type → "blanket trait impl is not allowed for user code").

Repro (`ext_copy.ark`): the #945 repro with the struct named `S` instead
of `Holder`. Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`) on both targets:

```
error[E0216|typecheck]: blanket trait impl is not allowed for user code: Copy
```

## Expected (ADR-089, 2026-10-27)

Impl type parameters are introduced only by an explicit binder
(`impl<T, U: Bound> …`), for trait and inherent impls alike. Every other
name in an impl head resolves as an ordinary type name; an unresolved name
is an undefined-type diagnostic that suggests adding a binder. Name length
or case never decides type vs. type parameter. The implicit form goes
through a migration warning before it becomes an error (ADR-014).

## Acceptance

- [x] The rule is decided: ADR-089 (explicit binder required).
- [ ] Binder-less impl heads with implicit parameters produce a migration
      warning that shows the binder form; the warning becomes an error after
      the migration (ADR-014).
- [ ] stdlib / selfhost impl heads that rely on the implicit form are
      migrated to binders (`impl … for Ctor<T>` and inherent `impl Ctor<T>`
      under `std/` and `src/compiler/`).
- [ ] `impl Trait for S {}` with `struct S` resolves to the struct (fixture).
- [ ] Existing parametric-impl fixtures stay green or are migrated to the
      binder form.
