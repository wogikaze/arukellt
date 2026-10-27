---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 946
Track: typecheck
Depends on: none
Orchestration class: design-then-implement
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-062, ADR-078, #945"
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

## Expected

A name that resolves to a declared type is that type. Type parameters are
introduced only by an explicit binder (`impl<T> …`), or the implicit rule
applies only to names that do not resolve to a declared type.

## Acceptance

- [ ] An ADR-062/078 amendment (or a new ADR) fixes the rule: explicit binder
      required, or resolution-first with the implicit rule as fallback, plus
      a migration diagnostic for the old form.
- [ ] `impl Trait for S {}` with `struct S` resolves to the struct (fixture).
- [ ] Existing parametric-impl fixtures stay green or are migrated to the
      binder form.
