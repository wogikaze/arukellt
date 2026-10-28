---
Status: open
Created: 2026-10-27
Updated: 2026-10-28
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
      Implementation note (2026-10-28): the final hard-error state landed as
      E0222. The ADR-014 warning window was not run as a separate release
      because the in-tree corpus was migrated atomically; this item stays open
      until that window is run or explicitly waived.
- [x] stdlib / selfhost impl heads that rely on the implicit form are
      migrated to binders (`impl … for Ctor<T>` and inherent `impl Ctor<T>`
      under `std/` and `src/compiler/`).
- [x] `impl Trait for S {}` with `struct S` resolves to the struct (fixture:
      `tests/fixtures/trait/single_uppercase_impl_target.ark`).
- [x] Existing parametric-impl fixtures stay green or are migrated to the
      binder form (80 heads under `std/` and `tests/fixtures/`).

## Implementation (2026-10-28)

- `src/compiler/typechecker/impl_generics.ark`: `declared_type_param_names`
  is the only impl parameter source; the implicit single-uppercase collector
  is removed. `undefined_head_type_names` finds head names that neither the
  binder nor a declaration in the compilation unit introduces.
- `src/compiler/typechecker/trait_registry.ark` and `module_traits.ark`:
  report E0222 with the binder form for trait and inherent impls. The trait
  head's base name resolves as a trait; only its type arguments are scanned.
- `src/compiler/resolver/impl_target.ark` and
  `src/compiler/corehir/decl_impl_target.ark`: impl owner keys treat only
  binder-declared names as parameters, so a concrete single-uppercase type
  argument receives its own instantiation key.
- `src/compiler/mir/lower/core_bindings.ark` and
  `core_bindings_let_store.ark`: `let b = a` keeps a one-character nominal
  storage type instead of dropping it as a shape-based type variable.
- Migrated 80 binderless parametric impl heads under `std/` (59) and
  `tests/fixtures/` (21). `src/compiler/` had no implicit heads.
- Fixtures: `trait/single_uppercase_impl_target.ark` (run),
  `diagnostics/implicit_impl_type_parameter.ark` (E0222, trait impl), and
  `diagnostics/implicit_inherent_impl_type_parameter.ark` (E0222, inherent
  impl).

### Verification (2026-10-28)

- `python3 scripts/manager.py selfhost build-compiler`: PASS
  (s2 sha256 `5181fb159131bd22db8b0446e301b2f839c5277135be526300d7bf9fbc7c4402`).
- `python3 scripts/manager.py verify fixtures`: PASS=1907 FAIL=0
  COMPILE_VALIDATE=1907 EXECUTED=109 RUNTIME_UNSUPPORTED=2; test: PASS=1
  FAIL=0.
- `python3 scripts/manager.py selfhost diag-parity`: PASS (all 162 `diag:`
  fixtures, including the two new E0222 goldens).
- `python3 scripts/manager.py verify lane`: PASS (`quality changed`, lint
  ratchet, touched-code quality ratchet).
- `python3 scripts/manager.py docs check`: 6/6 PASS.
- `python3 scripts/manager.py selfhost fixpoint`: PASS, sha256(s2) ==
  sha256(s3) = `5181fb159131bd22db8b0446e301b2f839c5277135be526300d7bf9fbc7c4402`
  (pinned base `8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee`).
- Both-target runtime evidence for `trait/single_uppercase_impl_target.ark`:
  `wasm32-gc` component prints `14`; `wasm32` linear core prints `14`.
- `check` on both E0222 fixtures emits the binder hint on the impl head
  (`undefined type `T`…` / `undefined type `Q`…`).

### Residuals

- E0222 covers only the removed single-uppercase spelling in impl heads.
  General unresolved type names in signatures and bodies remain #939.
- The one-character shape heuristic remains in the return-type and
  type-variable encoding paths (`corehir/return_type_name.ark`,
  `mir/lower/mono_suffix_parse.ark`, `corehir/param_shape_*.ark`,
  `mir/lower/stmt_let_tuple.ark`). Returning a single-uppercase struct from a
  function can still miscompile; plain functions already had this defect, and
  impl methods become reachable after this change. Follow-up is required
  before claiming full single-uppercase nominal-type support.
- The ADR-014 migration-warning window was not run; E0222 is an error.
