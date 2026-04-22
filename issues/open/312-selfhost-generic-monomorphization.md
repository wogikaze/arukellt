# Selfhost に generic instantiation と monomorphization を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-04-18
**ID**: 312
**Depends on**: 311
**Track**: selfhost-frontend
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v5**: yes
**Priority**: 11
**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

## Reopened by audit — 2026-04-13

**Reason**: No monomorphization pass.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Consistency audit — 2026-04-18

**Why**: Acceptance checkboxes were all `[x]` while the summary and reopen note still said there is no selfhost monomorphization pass. Repository state is in between: the selfhost typechecker has partial generic call inference and records `mono_instances`, but MIR lowering does not consume them and there is no specialization pass before codegen.

**Evidence (selfhost)**:

- `instantiate_type` only substitutes a top-level `TY_TYPE_VAR`; it does not walk `TypeInfo.type_args`, so parameters like `Vec<T>` do not get `T` replaced when checking generic calls (`src/compiler/typechecker.ark`, `instantiate_type`).
- `record_mono_instance` runs for generic `NK_CALL` callees but not for generic `NK_METHOD_CALL` (`infer_expr`, same file).
- `mir::lower_to_mir` takes `TypeCheckResult` but only uses `typed_fns` (return-type tags); `mono_instances` is never read (`src/compiler/mir.ark`, `lower_to_mir`).
- `has_mono_instance_for` is currently unused (dead helper).

**Evidence (Rust toolchain)**:

- Monomorphization / generic lowering live under `crates/ark-mir/src/lower/` (e.g. `func.rs`, `ctx.rs`, `expr.rs`). `lower/mod.rs` is module wiring and re-exports only, not where specialization logic sits.

## Summary

型パラメータの具体化と monomorphization pass を実装する。#308 で parse した generic 宣言を typechecker で instantiate し、backend に渡す前に具象型に展開する。Vec<i32> と Vec<String> を異なる具象関数/型として扱えるようにする。

## Current state

- `src/compiler/typechecker.ark`: unification (`bind_var` / `resolve_type`), generic-aware annotations (`resolve_type_ann_node_generic`), and call-site inference for generic functions exist; `mono_instances` records mangled names for **direct** generic calls only. Shallow `instantiate_type` limits correctness when type parameters appear nested (e.g. inside `Vec<T>`).
- `src/compiler/hir.ark`: generic parameter metadata exists on HIR shapes; full concrete expansion before codegen is still missing.
- **No** selfhost monomorphization pass: no phase duplicates generic items or rewrites call targets using `mono_instances` before MIR/Wasm.
- Rust: `crates/ark-mir` (lowering submodules, see crate doc comment in `crates/ark-mir/src/lib.rs`) performs monomorphization-style specialization on the non-selfhost pipeline.

## Acceptance

- [ ] `Vec<i32>` と `Vec<String>` が異なる具象型として扱われる — **Open**: distinct `type_args` unify for concrete uses; nested generic parameters are not fully instantiated (`instantiate_type`); MIR does not specialize on `mono_instances`.
- [ ] generic fn の呼び出しで型引数が推論される — **Open**: partial for `NK_CALL`; generic `NK_METHOD_CALL` does not record monomorph instances; shallow instantiation as above.
- [x] monomorphization 後の typed function list が backend に渡される — **Done (slice-c)**: `lower_to_mir` now emits one specialized `MirFunction` per `MonoInstance` and rewrites generic call sites (via per-call-site `mono_call_sites` span map) to dispatch to the mangled specialization. Generic-source bodies with ≥1 instantiation are skipped from `module.functions`.
- [ ] 未使用の generic instantiation が codegen に含まれない — **Open**: depends on a real specialization / reachability pass; not implemented for selfhost.

## References

- `src/compiler/typechecker.ark` — TypeInfo, unification, `mono_instances`, `instantiate_type`
- `src/compiler/mir.ark` — `lower_to_mir`, `TypeCheckResult.mono_instances` (unused)
- `src/compiler/hir.ark` — HIR generic parameter
- `crates/ark-mir/src/lower/` — Rust MIR lowering / specialization (not `mod.rs` alone)
- `crates/ark-typecheck/src/checker/` — Rust generic instantiation

## Status (slice-a, 2026-04-22)

**Done in this slice (nested generic instantiation correctness):**

- `instantiate_type` is now fully recursive over composite shapes
  (walks `TypeInfo.type_args` for non-`TY_TYPE_VAR` heads); the prior
  audit note about a "top-level only" substitution is now stale
  (`src/compiler/typechecker.ark`, `instantiate_type`).
- Made `mono_type_key` recurse into `type_args` so that
  `f<Vec<i32>>` and `f<i32>` (and any other distinct nested
  concretizations) produce distinct mangled names rather than
  collapsing into `f__Vec` / `f__i32` heads only
  (`src/compiler/typechecker.ark`, `mono_type_key`).
- Added `resolve_type_deep` and routed both the `NK_CALL` and
  `NK_METHOD_CALL` monomorph-recording sites through it, so any nested
  fresh type variables that end up bound during call-site inference
  are fully resolved before being handed to `record_mono_instance`
  (`src/compiler/typechecker.ark`, `resolve_type_deep`,
  `infer_expr` NK_CALL / NK_METHOD_CALL arms).

**Evidence:**

- New runtime fixture exercising a nested generic call:
  `tests/fixtures/generics_v1/nested_generic_call.ark` (registered in
  `tests/fixtures/manifest.txt` under `run:`, `t3-compile:`, and
  `t3-run:` sections). Calls `count<i32>(Vec<i32>)` and
  `count<Vec<i32>>(Vec<Vec<i32>>)` — these are now distinct
  `mono_instances` entries with mangled keys `count__i32` and
  `count__Vec<i32>`.
- All four canonical selfhost gates green:
  - `python3 scripts/manager.py selfhost fixpoint` → PASS
  - `python3 scripts/manager.py selfhost fixture-parity` → PASS
  - `python3 scripts/manager.py selfhost parity --mode --cli` → PASS
  - `python3 scripts/manager.py selfhost diag-parity` → PASS

**Remaining acceptance bullets (still Open — separate slices):**

- `Vec<i32>` と `Vec<String>` が異なる具象型として扱われる — partial:
  typechecker now distinguishes them in `mono_instances`; MIR
  specialization on `mono_instances` is still pending.
- generic fn の呼び出しで型引数が推論される — partial: `NK_CALL` and
  `NK_METHOD_CALL` both record monomorph instances now; an
  end-to-end nested-method-call regression fixture is still pending.
- monomorphization 後の typed function list が backend に渡される —
  `lower_to_mir` still ignores `mono_instances`; backend still sees one
  MIR function per generic source decl. (slice-c)
- 未使用の generic instantiation が codegen に含まれない — depends on
  real specialization + reachability pass. (slice-d)

Issue remains **open**. Three acceptance bullets still pending.

## Status (slice-c, 2026-04-22)

**Done in this slice (MIR-level monomorphization):**

- Extended `MonoInstance` with concrete `type_args: Vec<TypeInfo>` and added a
  per-call-site `MonoCallSite { span_start, mangled_name }` recording so MIR
  can rewrite generic call targets without re-running inference. Both lists
  are exposed on `TypeCheckResult` (`src/compiler/typechecker.ark`,
  `MonoInstance`, `MonoCallSite`, `record_mono_call`).
- `lower_to_mir` now consumes both lists (#312 acceptance bullet 3):
  - Pre-registers each specialized name (wasm-safe form via
    `mono_safe_name` — replaces `<>,` with `_`) into `fn_names`/
    `fn_return_vts`/`fn_return_type_names`, inheriting the generic
    source's return-type metadata so call dispatch finds the
    specialized target.
  - Skips emitting the generic source `MirFunction` whenever the source
    has ≥1 recorded `MonoInstance` (only specialized forms reach
    codegen; matches the issue's "non-emitted stub" guidance).
  - Post-pass after the main lower loop re-runs the per-decl lowering
    pipeline once per `MonoInstance`, producing one `MirFunction` per
    concrete instantiation with `name = mono_safe_name(mangled)`.
  - At every `NK_CALL` emit, `ctx_mono_lookup_call` keys by call AST
    `node.span.start` and rewrites `call_inst.str_val` to the
    specialized mangled name when the typechecker recorded one for that
    site (`src/compiler/mir.ark`).

**Evidence:**

- New runtime fixture exercising distinct concrete instantiations
  (`Vec<i32>` vs `Vec<String>`) of one generic function:
  `tests/fixtures/generics_v1/mir_specialization.ark` (registered in
  `tests/fixtures/manifest.txt` under `run:`, `t3-compile:`, and
  `t3-run:` sections).  After slice-c the MIR module exposes
  `count_items__Vec_i32_` and `count_items__Vec_String_` as separate
  functions and the two `count_items(...)` call sites in `main`
  dispatch to the matching specialization.
- All four canonical selfhost gates green:
  - `python3 scripts/manager.py selfhost fixpoint` → PASS
  - `python3 scripts/manager.py selfhost fixture-parity` → PASS
  - `python3 scripts/manager.py selfhost diag-parity` → PASS
  - `python3 scripts/manager.py selfhost parity --mode --cli` → PASS

**Remaining acceptance bullets (still Open — slice-d and follow-ups):**

- `Vec<i32>` と `Vec<String>` が異なる具象型として扱われる — partial:
  the typechecker distinguishes them in `mono_instances`, the MIR layer
  now emits distinct specialized functions per instantiation, and call
  sites resolve to them.  Remaining work: deeper substitution of `T`
  inside specialized bodies (e.g. propagating concrete vec elem types
  through every local for backend-specific dispatch beyond the
  `vec:T → vec_*` shape) is left for a follow-up; the body that this
  slice emits is the type-erased form of the generic source.
- generic fn の呼び出しで型引数が推論される — slice-a covered
  recording; an end-to-end nested-method-call regression fixture is
  still pending.
- 未使用の generic instantiation が codegen に含まれない — depends on
  real reachability pass. (slice-d)
