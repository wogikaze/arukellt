---
Status: open
Created: 2026-04-14
Updated: 2026-05-16
ID: 495
Track: selfhost
Depends on: 312, 504
Orchestration class: implementation-ready
Orchestration upstream: —
---

# 495 — Selfhost typechecker: trait bounds and constraint solving

## Summary

Issue #210 shipped basic typed-function typechecking but explicitly deferred
trait bounds and constraint solving. No open issue tracks this for the selfhost
compiler.

## Depends on

- #312 (selfhost generic monomorphization) — **DONE** ✅
- #504 (trait/interface syntax and impl-block infrastructure) — **DONE** ✅

## Assessment: 2026-05-16

### Dependency verification

**#312**: All acceptance criteria satisfied. Source contains `MonoInstance`, `MonoCallSite`, `instantiate_type` (fully recursive), `mono_type_key` (recursive), `resolve_type_deep`. MIR lowering consumes mono instances with call-site rewrites and `mir_prune_unreachable`. Selfhost fixpoint, fixture parity, diag parity all pass. Pinned bootstrap refreshed (sha256 `341e645e`).

**#504**: All 6 acceptance criteria satisfied. Source contains:
- `register_trait_impl`, `type_satisfies_trait_bound`, `enforce_trait_bound`, `enforce_fn_sig_bounds`
- `TypeParamBounds` struct and `collect_type_param_bounds`, `collect_type_param_names`
- `FnSig.type_param_bounds` field (populated by `fn_sig_from_fn_decl`)
- `maybe_register_impl_decl` called in `typecheck_module` pass 1
- `enforce_fn_sig_bounds` called at both `NK_CALL` (line 954) and `NK_METHOD_CALL` (line 1024) generic paths
- Parse fixtures: `parser_hir_trait_bounds_smoke.ark`, `parser_new_syntax.ark`
- Typecheck fixture: `typecheck_trait_impl_smoke.ark` (passes fixture parity)
- Additional fixture: `generics_v1/trait_bound.ark` (traits with struct methods + bounded generic call)

All source paths: `src/compiler/typechecker.ark`

### Blocker analysis

**No upstream blocker exists.** Both declared dependencies (312, 504) are DONE and their code is fully present in the source tree. The prior `blocked-by-upstream` classification was stale.

The issue is **implementation-ready**: the core typechecker machinery (trait impl registration, bound extraction, bound enforcement at call sites) was delivered by #504. What remains is verification and fixture coverage.

### Remaining gaps

1. **Negative fixture**: No test exists that proves the typechecker rejects unsatisfied trait bounds (e.g., calling `fn f<T: Foo>(x: T)` with a type that has no impl).
2. **Acceptance criteria unchecked**: All 4 items are still `[ ]` — they need verification.
3. **`cargo test`**: Not verified in this assessment.
4. **`verify quick`**: Currently 20/22 pass (2 pre-existing failures: doc example check in `lang-uplift-gap-ledger.md`, broken internal links). These are unrelated to #495.

### Current state

- `type_satisfies_trait_bound` returns `true` for unresolved `TY_TYPE_VAR` (correct deferral) and checks `env.trait_impls` for concrete types.
- `maybe_register_impl_decl` registers `impl Trait for Type` entries via `type_ann[1]` (self type).
- The positive fixture `typecheck_trait_impl_smoke.ark` passes selfhost fixture parity.
- `verify quick` passes at 20/22 (pre-existing infra failures only).

## Primary paths

- `src/compiler/typechecker.ark`

## Non-goals

- Higher-kinded types
- Trait coherence / orphan rules (language-level design, not selfhost scope)

## Acceptance

- [ ] Selfhost typechecker resolves trait bounds on generic parameters — **partial: machinery exists, needs positive-fixture verification**
- [ ] Selfhost typechecker rejects programs with unsatisfied trait bounds — **needs negative fixture**
- [ ] At least one positive and one negative fixture exercise trait-bound checking — **positive fixture exists, negative needed**
- [ ] `cargo test` passes

## Required verification

- `python scripts/manager.py verify quick` passes

## Close gate

Acceptance items checked; fixtures prove trait-bound acceptance and rejection.
