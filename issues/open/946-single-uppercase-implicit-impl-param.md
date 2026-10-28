---
Status: open
Created: 2026-10-27
Updated: 2026-10-29
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

### Residual removal (2026-10-29)

The return and tuple encoding paths no longer decide type-variable status
from a one-character name; the parameter path and its evidence are recorded
below:

- `corehir/frontend_fns.ark` adds `fn_type_param_is_declared`, which compares
  a spelling against the function's declared type parameters.
- `corehir/return_type_name.ark` and `corehir/return_type_value.ark` consult
  the declared binder names. A single-uppercase name that is not declared is
  a concrete type, so an impl method returning `struct S` keeps `S` as its
  return name and GC reference value type.
- `corehir/param_shape_*.ark` was not changed. A binder-based reclassification
  of the initial parameter type changed the `enums/i8_payload.ark` signature
  for `print_value(e: E)` from the enum-open `(ref null 14)` to `structref`
  and failed wasm validation. The existing parameter path already resolves a
  declared `struct S` parameter through the registered layout, and the new
  fixture covers free-function and method parameters. The shape test in
  `param_shape_value::param_ann_name_is_type_param` and the local-type-name
  selection in `param_shape_local` therefore remain residuals.
- `mir/lower/fn_index_method_return_type.ark` treats every non-`?` method
  return name as concrete. A declared type parameter is already spelled empty
  by `return_index_type_name`, so a remaining name is a nominal type.
- `wasm/sections_types_gc_plan.ark` resolves a legacy generic struct name
  against the registered GC struct table before falling back to `structref`.
- `mir/lower/aggregate_tuple.ark` and `mir/lower/stmt_let_tuple.ark` use the
  declared aggregate registry: a declared `S` tuple element gets the named
  `gcsref` field layout, and its destructured local keeps the GC reference
  value type instead of an i32 placeholder.

Fixture: `functions/single_uppercase_struct_return.ark` (run + t3-run,
expected `70`) returns `S` from a free function and an impl method, passes it
through free-function and method parameters, and destructures `(S, i32)`.

Verification (2026-10-29):

- `python3 scripts/manager.py selfhost build-compiler`: PASS, elapsed
  1835.3s, final s2 sha256
  `1e10f559f16ae15e99bf3300657668c103ecd486ce703d6f4e669173235ebe7e`
  (pinned base `8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee`).
- `python3 scripts/manager.py verify fixtures`: PASS=1882 FAIL=0
  COMPILE_VALIDATE=1882 EXECUTED=109 RUNTIME_UNSUPPORTED=2;
  test: PASS=36 FAIL=0.
- `python3 scripts/manager.py verify lane`: PASS (`quality changed`, lint
  checked=10 failed=0). The advisory touched-code report lists
  `return_type.ark thin_wrappers 4 > 3` because the new impl-return tag
  wrapper mirrors the existing fn-return wrapper; the global ratchet passes.
- `python3 scripts/manager.py docs check`: 6/6 PASS.
- `python3 scripts/manager.py selfhost fixpoint`: PASS, sha256(s2) ==
  sha256(s3) = `1e10f559f16ae15e99bf3300657668c103ecd486ce703d6f4e669173235ebe7e`.
- Both-target runtime evidence for
  `functions/single_uppercase_struct_return.ark`: `wasm32-gc` component prints
  `70`; `wasm32` linear core prints `70`.
- The first build of this lane classified enum parameters through the binder
  and failed `enums/i8_payload.ark` (`print_value(e: E)` signature changed
  from `(ref null 14)` to `structref`). After reverting that parameter-path
  experiment, the rebuilt s2 passes `verify fixtures` with 1882/0.

### Parameter and mono residual removal (2026-10-29)

Declaration evidence now decides the parameter/local encoding before the
one-character shape fallback:

- `corehir/param_shape_declared.ark` builds the declared struct/enum name
  table for the compilation unit and owns the declared-binder comparison.
- `corehir/param_shape_value.ark` and `corehir/param_shape_local.ark` accept
  the function/impl-method binder list and the declared nominal table. A
  declared type parameter is a placeholder; a declared nominal type stays
  concrete at one character. Declared enums keep the canonical `enum:<name>`
  local spelling, so an enum parameter keeps the open-enum type through mono
  substitution (the regression the previous binder retry hit).
- `mir/lower/params_method.ark` preserves that `enum:` spelling instead of
  resolving it as a named struct.
- `mir/lower/mono_param_subst.ark` and
  `mir/lower/ctx_mono_type_params.ark` skip type-variable substitution and
  ordinal collection for names registered as structs/enums.
- `wasm/sections_types_sigs_mono.ark` treats a registered struct/enum return
  name as concrete instead of consuming the mono suffix key.
- `wasm/sections_types_gc_plan.ark` resolves a registered enum base to the
  open-enum signature before the legacy `structref` fallback.

Fixtures (run + t3-run): `functions/single_uppercase_enum_return.ark`
(expected `7`), `enums/single_uppercase_generic_impl_param.ark` (expected
`6`), and `generics_v1/generic_impl_single_uppercase_struct.ark` (expected
`43`). Baseline failures before the change: the enum-return free function
registered a `structref` result while the call site used the open-enum local
(`func ... expected (ref null $type), found structref`), and the generic
struct method emitted its `S` parameter as `i32`/`i64` instead of the struct
reference (`expected (ref null $type), found i64`).

Verification (2026-10-29):

- `python3 scripts/manager.py selfhost build-compiler`: PASS, elapsed
  2046.4s, s2 sha256
  `1a170eee2669235974179b7e106b0e9ac9881b606e9d51cf85a11aa2e2fe6445`.
- `python3 scripts/manager.py verify fixtures`: PASS=1886 FAIL=0
  COMPILE_VALIDATE=1886 EXECUTED=109 RUNTIME_UNSUPPORTED=2;
  test: PASS=36 FAIL=0.
- `python3 scripts/manager.py verify lane`: PASS; lint checked=17 failed=0.
  The advisory touched-code report lists `frontend_fns.ark thin_wrappers
  1 > 0 (HEAD)` for the declared-parameter comparison facade; the global
  ratchet passes.
- `python3 scripts/manager.py docs check`: 6/6 PASS.
- `python3 scripts/manager.py selfhost fixpoint`: PASS, sha256(s2) ==
  sha256(s3) =
  `1a170eee2669235974179b7e106b0e9ac9881b606e9d51cf85a11aa2e2fe6445`
  (pinned base `8df2698c8401693879ca4f9ced289ed2c285d95eb526cf70fda02dd5876259ee`).
- Both-target runtime evidence: the `wasm32-gc` component and the `wasm32`
  linear core print `7`, `6`, and `43` for the three fixtures.

### Residuals

- E0222 covers only the removed single-uppercase spelling in impl heads.
  General unresolved type names in signatures and bodies remain #939.
- `corehir/param_shape_*.ark` and the mono substitution/collection paths now
  consult declared binders, declared nominal names, and registered layouts
  before the shape test. `mir/lower/mono_suffix_parse.ark::is_mono_type_var_name`
  keeps the single-uppercase/ordinal shape test only as the last fallback for
  a spelling with no declaration or registered layout.
- A generic method whose declared return is its own type parameter
  (`fn pass<U>(value: U) -> U`) is still not specialized when `U` is a user
  struct: both `S` and `Point` fail W0004 (`expected anyref, found i64`).
  This is a generic return-substitution gap, not a one-character shape case,
  and is not fixed here.
- The ADR-014 migration-warning window was not run; E0222 is an error.
