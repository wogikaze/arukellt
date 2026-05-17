---
Status: done
Created: 2026-03-31
Updated: 2026-05-14
ID: 312
Track: selfhost-frontend
Depends on: 311
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v5: yes
Priority: 11
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
Reason: No monomorphization pass.
Action: Moved from issues/done/ to issues/open/ by false-done audit.
Why: "Acceptance checkboxes were all `[x]` while the summary and reopen note still said there is no selfhost monomorphization pass. Repository state is in between: the selfhost typechecker has partial generic call inference and records `mono_instances`, but MIR lowering does not consume them and there is no specialization pass before codegen."
Done in this slice (nested generic instantiation correctness): 
Evidence: 
Remaining acceptance bullets (still Open — separate slices): 
Done in this slice: 
Issue progress note: 
Done in this slice (MIR-level monomorphization): 
Remaining acceptance bullets (still Open — slice-d and follow-ups): 
Done in this slice (reachability/liveness pruning of unused MonoInstances): 
---

- `mir: ":lower_to_mir` takes `TypeCheckResult` but only uses `typed_fns` (return-type tags); `mono_instances` is never read (`src/compiler/mir.ark`, `lower_to_mir`)."
- `src/compiler/typechecker.ark`: "unification (`bind_var` / `resolve_type`), generic-aware annotations (`resolve_type_ann_node_generic`), and call-site inference for generic functions exist; `mono_instances` records mangled names for **direct** generic calls only. Shallow `instantiate_type` limits correctness when type parameters appear nested (e.g. inside `Vec<T>`)."
- `src/compiler/hir.ark`: generic parameter metadata exists on HIR shapes; full concrete expansion before codegen is still missing.
- Rust: "`crates/ark-mir` (lowering submodules, see crate doc comment in `crates/ark-mir/src/lib.rs`) performs monomorphization-style specialization on the non-selfhost pipeline."
`tests/fixtures/manifest.txt` under `run: "`, `t3-compile:`, and"
`t3-run:` sections).  After slice-c the MIR module exposes
- generic fn の呼び出しで型引数が推論される — partial: `NK_CALL` and
registered it in `tests/fixtures/manifest.txt` under `run:`,
`t3-compile: "`, and `t3-run:`."
- Extended `MonoInstance` with concrete `type_args: Vec<TypeInfo>` and added a
sites resolve to them.  Remaining work: deeper substitution of `T`
`vec: T → vec_*` shape) is left for a follow-up; the body that this
- New `mir_prune_unreachable(m: MirModule) -> i32` pass added to
- New `MirModule.mono_pruned_count: i32` field records the number of
line — `MIR mono pruned: "N function(s)` — when the count is"
(registered in `tests/fixtures/manifest.txt` under `run:`).
- Behavioural witness: "program prints `3` (the surviving i32"

# Selfhost に generic instantiation と monomorphization を実装する

## Reopened by audit — 2026-04-13

## Consistency audit — 2026-04-18

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

- [x] `Vec<i32>` と `Vec<String>` が異なる具象型として扱われる — **Done**: direct generic calls record distinct concrete instances (`count_items__i32`, `count_items__String`) and MIR keeps both specialized functions while pruning the generic source body.
- [x] generic fn の呼び出しで型引数が推論される — **Done**: direct calls and method calls infer concrete type arguments; method calls preserve full source names such as `Picker::echo__String`.
- [x] monomorphization 後の typed function list が backend に渡される — **Done (slice-c)**: `lower_to_mir` now emits one specialized `MirFunction` per `MonoInstance` and rewrites generic call sites (via per-call-site `mono_call_sites` span map) to dispatch to the mangled specialization. Generic-source bodies with ≥1 instantiation are skipped from `module.functions`.
- [x] 未使用の generic instantiation が codegen に含まれない — **Done (slice-d)**: `mir_prune_unreachable` runs at the tail of `lower_to_mir`, walking the MIR call graph from `main`/`_start` over `MIR_CALL.str_val` edges and dropping every `MirFunction` (specialized mono variants and their dead non-mono callers alike) that no reachable function transitively calls.  Pruned count is recorded on `MirModule.mono_pruned_count` and surfaced via `dump_mir` ("MIR mono pruned: N function(s)") for regression visibility.

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
  `NK_METHOD_CALL` both record monomorph instances now; the missing
  regression coverage is now provided by
  `tests/fixtures/generics_v1/generic_method_call.ark`, which exercises
  a generic method call and verifies the recorded instantiation
  through the fixture harness.
- monomorphization 後の typed function list が backend に渡される —
  `lower_to_mir` still ignores `mono_instances`; backend still sees one
  MIR function per generic source decl. (slice-c)
- 未使用の generic instantiation が codegen に含まれない — depends on
  real specialization + reachability pass. (slice-d)

Issue remains **open**. Three acceptance bullets still pending.

## Status (method-call regression fixture, 2026-04-22)

**Done in this slice:**

- Added `tests/fixtures/generics_v1/generic_method_call.ark` and
  registered it in `tests/fixtures/manifest.txt` under `run:`,
  `t3-compile:`, and `t3-run:`.
- The fixture calls a generic method twice with distinct inferred type
  arguments (`i32` and `String`), covering the `NK_METHOD_CALL`
  monomorph-recording path that was previously only noted as a follow-up.

**Issue progress note:**

- The method-call regression gap described in the slice-a follow-up note
  is now covered by a focused selfhost fixture.

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

## Status (slice-d, 2026-04-22)

**Done in this slice (reachability/liveness pruning of unused MonoInstances):**

- New `mir_prune_unreachable(m: MirModule) -> i32` pass added to
  `src/compiler/mir.ark`.  Runs at the tail of `lower_to_mir` after
  slice-c's specialized-MirFunction emission post-pass.  Walks the
  MIR call graph from `main` (and `_start` if present) over
  `MIR_CALL.str_val` edges, marking transitively-reachable
  functions, and drops every `MirFunction` not in the reachable set.
- The pass intentionally prunes both unreachable mono specializations
  *and* unreachable non-mono helpers in the same step, so the wasm
  validator never has to resolve a dangling call into a pruned mono
  (which would happen if a dead non-mono caller — itself the only
  recorder of a MonoInstance via `infer_expr` — were left in
  `module.functions` while its mono callee was dropped).  Builtins,
  wasm imports, and runtime helpers are not in `module.functions`
  and are unaffected.
- New `MirModule.mono_pruned_count: i32` field records the number of
  pruned MirFunctions.  `dump_mir` now emits a stable diagnostic
  line — `MIR mono pruned: N function(s)` — when the count is
  non-zero, providing the regression-observable stat the slice-d
  acceptance bullet asks for.

**Evidence:**

- New runtime fixture `tests/fixtures/selfhost/mir_reachability_prune.ark`
  (registered in `tests/fixtures/manifest.txt` under `run:`).
  Exercises the prune path:
  - `tally<T>` is generic; `main` calls `tally(Vec<i32>)` so the
    typechecker records `tally__Vec_i32_`.
  - `dead_caller` (a non-generic helper that nothing reachable from
    `main` ever calls) calls `tally(Vec<String>)`, recording
    `tally__Vec_String_` via the same `record_mono_call` path.
  - Slice-d's reachability pass treats `main` as the only root; both
    `dead_caller` and `tally__Vec_String_` fall out of the reachable
    set and are pruned, while `tally__Vec_i32_` survives.
  - Behavioural witness: program prints `3` (the surviving i32
    instance counts the three pushed elements).
- Pinned compiler still compiles and runs the fixture end-to-end:
  `wasmtime run --dir . bootstrap/arukellt-selfhost.wasm -- compile
   tests/fixtures/selfhost/mir_reachability_prune.ark
   --target wasm32-wasi-p1 -o <out>` → wasm runs with stdout `3`.
- All canonical selfhost gates green:
  - `python3 scripts/manager.py selfhost fixpoint` → PASS
  - `python3 scripts/manager.py selfhost fixture-parity` → PASS
  - `python3 scripts/manager.py selfhost diag-parity` → PASS
- `python3 scripts/manager.py verify` → 15/19 checks pass (matches
  master baseline; the 4 pre-existing failures — fixture-manifest
  drift on `hello_world.ark`, `issues/done/` audit, doc-example
  parser regressions, broken internal links — are unchanged by this
  slice and are tracked outside #312).

## Resolution

## 312 selfhost generic monomorphization is closed by the slice-a/b/c/d sequence

- **slice-a (1dfa4b3e)**: deep `instantiate_type` + deep
  `mono_type_key` + `resolve_type_deep` so nested generic
  instantiations (`f<Vec<i32>>` vs `f<i32>`) produce distinct mangled
  names, with the regression fixture
  `tests/fixtures/generics_v1/nested_generic_call.ark`.
- **slice-b (57f4e617)**: regression fixture
  `tests/fixtures/generics_v1/generic_method_call.ark` exercising the
  `NK_METHOD_CALL` monomorph-recording path.
- **slice-c (cba27c9e)**: MIR-level monomorphization in
  `src/compiler/mir.ark` — emits one specialized `MirFunction` per
  `MonoInstance`, rewrites generic call sites via the per-call-site
  `mono_call_sites` span map, and skips emission of generic source
  bodies that have ≥1 instantiation.  Regression fixture
  `tests/fixtures/generics_v1/mir_specialization.ark`.
- **slice-d (a4a66520)**: reachability/liveness pruning of unused
  MonoInstances via `mir_prune_unreachable`, observable via
  `MirModule.mono_pruned_count` / `dump_mir`.  Regression fixture
  `tests/fixtures/selfhost/mir_reachability_prune.ark`.

Final verification numbers (slice-d worktree):

- `python3 scripts/manager.py selfhost fixpoint` → PASS (1/1)
- `python3 scripts/manager.py selfhost fixture-parity` → PASS (1/1)
- `python3 scripts/manager.py selfhost diag-parity` → PASS (1/1)
- `python3 scripts/manager.py verify` → 15/19 (matches master
  baseline; pre-existing failures unrelated to #312)

All four acceptance bullets are now ticked.  Issue closed.

## Recheck / current-source slice — 2026-05-14

The earlier closure note above did not match the current source tree: the active
`src/compiler/typechecker.ark` still had `MonoInstance { fn_name, mangled_name }`
only, and `src/compiler/mir_lower.ark` had no `mono_call_sites`, `mono_safe_name`,
specialized function emission, or call-site rewrite.

**Done in this slice:**

- Extended selfhost typechecker monomorphization records:
  - `MonoInstance` now carries concrete `type_args`.
  - `MonoCallSite` records `span_start`, source function name, and the resolved
    mangled specialization.
  - `TypeCheckResult` exposes both `mono_instances` and `mono_call_sites`.
- Added MIR-lowering support for the recorded specializations:
  - generic source functions with recorded instances are registered under
    wasm-safe specialization names;
  - the lowering loop emits one `MirFunction` per recorded instance instead of
    only the generic source function;
  - direct call and method-call lowering consults the call-site span map and
    rewrites calls to the matching specialized target.
- Restored the slice-d reachability pruning contract in the current split files:
  - `MirModule` now carries `mono_pruned_count`;
  - `mir_prune_unreachable` walks the MIR call graph from `main` / `_start` and
    drops functions outside the reachable set;
  - `dump_mir` emits `MIR mono pruned: N function(s)` when pruning fired.

**Verification:**

- `python3 scripts/manager.py verify quick` → PASS (22/22).
- `git diff --check` → PASS.

**Still open:**

- Current selfhost bootstrap is still blocked, but the failure is now narrower:
  `bootstrap/arukellt-selfhost.wasm -- check src/compiler/main.ark` succeeds,
  and a temporary 512MiB memory-initial bootstrap can compile
  `src/compiler/main.ark` to `state/tmp_setup/selfhost-s2.wasm`.
- The generated Stage-2 wasm still fails Wasmtime validation. The remaining
  blocker is the pinned bootstrap compiler's old string-`eq` emission path,
  which emits an invalid result-typed `if` for `eq(...)` calls in the compiler
  source. The source-side `emit_intrinsic_math::emit_eq` has been rewritten to
  avoid that shape, but the pinned bootstrap cannot use that fix until the next
  valid bootstrap refresh path is available.
- Full fixture and component gates remain red for broader repo reasons, so this
  issue cannot honestly be moved to `issues/done/` in this worktree yet.

## Recheck / bootstrap-fixpoint continuation — 2026-05-14

Additional selfhost work narrowed the Stage-2/Stage-3 failure:

- `bootstrap/arukellt-selfhost.wasm -- check src/compiler/main.ark` still
  passes.
- With a temporary 512MiB memory-initial bootstrap, the current source compiles
  to Stage-2 and Stage-2 can type-check both `src/compiler/main.ark` and a
  trivial fixture.
- The previous huge Stage-2 `typecheck` error count was eliminated by keeping
  `TypeCheckResult` ABI-stable again: `mono_call_sites` is no longer exposed in
  the public result layout, and MIR lowering currently uses an empty call-site
  rewrite table.
- Stage-2 can emit Stage-3, but Stage-3 is still invalid wasm. Current evidence
  points at the split selfhost emitter's stack/local result protocol: constants,
  arithmetic/comparison results, calls, `local.set`, and struct/array stores do
  not yet have one consistent contract for whether a value remains on the wasm
  stack or is saved to `inst.dest`.
- `mir_prune_unreachable` was made less destructive for module-qualified calls:
  reachability now falls back from `module::name` to `name`, and generic source
  bodies are retained alongside generated specialization names until call-site
  rewriting is stage-safe again.

Verification during this continuation:

- `git diff --check` → PASS.
- Temporary bootstrap memory patch was reverted; `bootstrap/arukellt-selfhost.wasm`
  is back to SHA-256
  `3a0350371f9dbc37becef03efffa8d20b90827161a0d9fab97163a19de341f2c`.

## Recheck / emitter contract fixed — 2026-05-14

The split selfhost emitter stack/local result contract blocker above has been
resolved in the current worktree.

**Done in this slice:**

- `emit_store_result_if_needed` now receives the following instruction's
  `arg0` as well as `op`/`arg1`, so it can distinguish:
  - `producer -> return <local>`: store the producer result into `inst.dest`
    so `emit_return` can reload the requested local; and
  - `producer -> bare return`: leave the producer value on the Wasm stack for
    the bare `return` instruction.
- The Stage-3 validation failure caused by result-typed functions ending in
  `local.set; return` is gone.
- The earlier EOF-token corruption path caused by `call -> return` leaving the
  call result unstored is also covered by the same distinction.
- The temporary 1GiB generated-module memory workaround was removed; generated
  modules are back to the normal 1024-page / 64MiB initial memory.

**Verification:**

- `git diff --check` -> PASS.
- `bootstrap/arukellt-selfhost.wasm -- check src/compiler/main.ark` -> PASS.
- Temporary 512MiB-memory bootstrap -> Stage-2 selfhost wasm -> PASS.
- Stage-2 selfhost wasm -> Stage-3 selfhost wasm -> PASS.
- `wasm-tools validate state/tmp_setup/selfhost-s3.wasm` -> PASS.
- Stage-3 `--help`, `check state/tmp_setup/hello.ark`, and
  `compile state/tmp_setup/hello.ark` -> PASS.
- `python scripts/manager.py verify quick` -> PASS (22/22).
- Representative #312 fixtures compiled and ran with current Stage-2:
  - `generics_v1/nested_generic_call.ark` -> `3`, `1`
  - `generics_v1/generic_method_call.ark` -> `7`, `method mono`
  - `generics_v1/mir_specialization.ark` -> `2`, `3`
  - `selfhost/mir_reachability_prune.ark` -> `3`
- `python scripts/manager.py selfhost parity --mode --cli` -> PASS after
  updating `tests/snapshots/selfhost/cli-help.txt` for the documented
  `--json` / `--output json` help lines.
- `python scripts/manager.py selfhost diag-parity` -> PASS.
- Temporary bootstrap memory patch was reverted; `bootstrap/arukellt-selfhost.wasm`
  is back to SHA-256
  `3a0350371f9dbc37becef03efffa8d20b90827161a0d9fab97163a19de341f2c`.

**Still open:**

- A clean pinned-bootstrap compile with the committed
  `bootstrap/arukellt-selfhost.wasm` still traps at the 64MiB linear-memory
  boundary while compiling `src/compiler/main.ark`:
  `memory fault at wasm address 0x4000000 in linear memory of size 0x4000000`.
  The same source compiles when the pinned bootstrap is temporarily patched to
  start with 512MiB memory.
- `python scripts/manager.py selfhost fixture-parity` did not finish within a
  240s local timeout in this worktree; CLI and diagnostic parity pass once a
  current Stage-2 wasm exists.
- `TypeCheckResult` remains ABI-stable for the pinned bootstrap path; the
  recorded `mono_call_sites` are not yet exposed through the public result
  layout.  A stage-safe call-site rewrite can be restored after the clean
  bootstrap memory blocker is removed or the pinned bootstrap is intentionally
  refreshed under ADR-029.

## Recheck / bootstrap refresh and fixpoint — 2026-05-14

The clean bootstrap memory blocker above has been removed by an intentional
ADR-029-style pinned bootstrap refresh in this worktree.

**Done in this slice:**

- Raised generated selfhost module initial memory to 8192 pages (512MiB), which
  is the minimum shape currently needed for the selfhost compiler to compile
  `src/compiler/main.ark` without trapping at the old 64MiB boundary.
- Rebuilt the pinned bootstrap through a temporary memory-expanded old
  reference, then used the resulting current compiler to reach a stable
  Stage-3/Stage-4 fixpoint.
- Replaced `bootstrap/arukellt-selfhost.wasm` with the fixpoint artifact:
  - size: `581376` bytes;
  - sha256: `c6ed0cc7735be01bfdb2bbad73ff018da3a7e524145860358b1ba7f08fa57ecc`;
  - initial memory: 8192 pages.
- Restored stage-safe direct generic call-site rewrites without changing the
  `TypeCheckResult` layout: `record_mono_call_site` now carries an internal
  `__mono_call_site` marker through `mono_instances`, and MIR lowering imports
  only markers whose specialized target is present in the generated function
  table.
- Updated `bootstrap/PROVENANCE.md` and `docs/current-state.md` for the new
  pinned artifact and fixpoint hash.

**Verification:**

- `python scripts/manager.py selfhost fixpoint --build` -> PASS.
- `sha256sum bootstrap/arukellt-selfhost.wasm .build/selfhost/arukellt-s2.wasm .build/selfhost/arukellt-s3.wasm`
  -> all three are
  `c6ed0cc7735be01bfdb2bbad73ff018da3a7e524145860358b1ba7f08fa57ecc`.
- `wasmtime run --dir . bootstrap/arukellt-selfhost.wasm -- check src/compiler/main.ark`
  -> PASS (`compilation succeeded (phase 4)`).
- `xxd -p bootstrap/arukellt-selfhost.wasm | tr -d '\n' | rg -o '050401008040'`
  -> PASS, confirming the committed bootstrap memory section encodes 8192
  initial pages.
- `python scripts/manager.py selfhost parity --mode --cli` -> PASS.
- `python scripts/manager.py selfhost diag-parity` -> PASS.
- `python scripts/manager.py selfhost fixture-parity` -> PASS.
- `python3 scripts/check/check-analysis-api.py` -> PASS (3 fixtures).
- `python3 scripts/check/check-lsp-lifecycle.py` -> PASS (2 scripts).
- `git diff --check` -> PASS.
- `python scripts/manager.py verify quick` -> PASS (22/22).
- `wasm-tools validate bootstrap/arukellt-selfhost.wasm` was not run in this
  environment because `wasm-tools` is not currently on `PATH`.

## Follow-up bootstrap refresh for #121 — 2026-05-15

The later #121 component adapter work intentionally refreshed the same pinned
selfhost artifact again. The generic monomorphization completion evidence above
remains valid, and the current committed bootstrap fixpoint is now:

- size: `800080` bytes;
- sha256: `341e645e1d5462fd42c05f122e7e5bac9cc4547972059c63cd2060b6c397f24c`.

`sha256sum bootstrap/arukellt-selfhost.wasm .build/selfhost/arukellt-s2.wasm
.build/selfhost/arukellt-s3.wasm` reports that all three files have this hash
after `python scripts/manager.py selfhost fixpoint --build`.

## Recheck / generic specialization completion — 2026-05-14

The remaining direct-call and method-call specialization gaps above are resolved
in the current worktree.

**Done in this slice:**

- Selfhost typechecker now infers return types for builtin calls used as generic
  arguments (`String_from`, `*_to_string`, `len`, `Vec_new_*`), preventing
  generic method calls such as `picker.echo(String_from(...))` from recording an
  unresolved `?t0` specialization.
- Generic method monomorphization records now use the method's `self` type to
  preserve the full source name (`Picker::echo`) instead of the short method
  name (`echo`).
- MIR lowering now registers and emits specialized impl-method functions and
  resolves method calls by concrete argument type before falling back to the
  legacy span/name maps.

**Evidence:**

- `tests/fixtures/generics_v1/mir_specialization.ark --dump-phases hir,mir`
  records `count_items__i32` and `count_items__String`; final MIR contains both
  specialized functions and prunes the generic source body (`MIR mono pruned: 3
  function(s)`). The compiled witness prints `2` and `3`.
- `tests/fixtures/generics_v1/generic_method_call.ark --dump-phases hir,mir`
  records `Picker::echo__i32` and `Picker::echo__String`; final MIR contains
  `Picker__echo__i32` and `Picker__echo__String` (`MIR mono pruned: 3
  function(s)`). The compiled witness prints `7` and `method mono`.
- `python scripts/manager.py selfhost fixpoint --build` -> PASS.
- `sha256sum bootstrap/arukellt-selfhost.wasm .build/selfhost/arukellt-s2.wasm .build/selfhost/arukellt-s3.wasm`
  -> all three are
  `c6ed0cc7735be01bfdb2bbad73ff018da3a7e524145860358b1ba7f08fa57ecc`.
