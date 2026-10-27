---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 933
Track: compiler
Depends on: "#934"
Orchestration class: architecture-implementation
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-040, ADR-053, RFC-002, #851, #934, #936, #939, #940, #941, #949"
---

# 933 — typechecker results are discarded before lowering; MIR re-infers types from strings

## Summary

The typechecker is not the owner of types after checking. `TypeCheckResult`
hands CoreHIR only diagnostics, typed function signatures, and mono
instances. Per-expression types, inferred local types, and method / trait /
overload resolution are dropped. MIR lowering then re-derives all of them
with a second, heuristic inference engine keyed on type-name strings and
callee names. The two engines can disagree, and the one that drives code
generation is the heuristic one.

This is the shared root cause of the miscompile and invalid-Wasm classes in
#935–#941 and of open bugs such as #928 (bare `clone` resolved to
`String::clone` through a name-keyed cache).

## Evidence (2026-10-27)

- `src/compiler/corehir/frontend_checked.ark` L18–48: the checked program is
  built from `typecheck_result_typed_fns` + `typecheck_result_mono_instances`
  only. `src/compiler/typechecker/contract_result_state.ark` has no accessor
  for expression / local types or resolved call targets.
- `src/compiler/corehir/frontend_typed_locals.ark` L18–31: "typed locals" are
  a syntactic re-scan of `let` annotations; an unannotated local defaults to
  `TypeInfo { tag: 1, name: "i32" }`.
- `src/compiler/corehir/frontend_resolved_calls.ark`: "resolved calls" are
  the callee text of every call node, collected syntactically. Nothing is
  resolved.
- MIR re-inference lives in `src/compiler/mir/lower/method_resolve.ark`,
  `call_type_fallback.ark`, `binary_type_refine.ark`, `expr_types.ark` and
  the post pass `src/compiler/mir/post_pass_type_propagate.ark`. The post
  pass is described as a fixpoint but is capped at `max_iterations = 2`
  (L62–66), tuned against the selfhost output rather than proven complete.
- `src/compiler/mir/post_pass_callee_lookup.ark` overrides local types from
  a callee-name cache; its own comment records that the cache "often maps
  bare clone to String and poisons GC struct temps" (#730, #928).
- Scale: `rg -c type_name src/compiler/mir` ≈ 1990 references versus ≈ 78
  for `TypeId`; `src/compiler/wasm` has ≈ 486 `starts_with(` checks on
  type / callee spellings (`"Option<"`, `"Option_"`, `"option:"`, …).

User-visible consequences are reproduced in #935–#941 (wrong output on
`wasm32`, invalid core Wasm on `wasm32-gc`).

## Expected

ADR-040 §1/§3: the semantic spine (TypeId / FunctionId / SignatureRegistry /
MonoInstanceTable) is the source of truth after checking, and lowering never
falls back to name archaeology. The typechecker is the only component that
infers types.

## Acceptance

- [ ] The typechecker exports, keyed by stable AST/CoreHIR node id: the
      TypeId of every expression and binding, and the resolved FunctionId /
      MonoInstanceId / impl of every call and method call (resolution input
      comes from #934).
- [ ] CoreHIR consumes that table; `frontend_typed_locals.ark`'s `i32`
      default and `scan_resolved_calls` are deleted.
- [ ] MIR lowering reads types and call targets only from the table;
      `method_resolve`, `call_type_fallback*`, `binary_type_refine` /
      `binary_type_select`, and `post_pass_type_propagate` /
      `post_pass_callee_lookup` are deleted (not kept as fallback).
- [ ] A missing type or target at lowering time is an internal compiler
      error, not a guess (ADR-040 §3).
- [ ] `MirLocal` carries a TypeId instead of `type_name: String` (aligns with
      ADR-053 Phase 4 "typed-by-construction").
- [ ] The repro fixtures registered by #935–#941 pass on both `wasm32` and
      `wasm32-gc`; `python3 scripts/manager.py verify fixtures` and
      `python3 scripts/manager.py selfhost fixpoint` stay green.

## Notes

- No bridge that re-derives types "where the table is empty" may land
  (`.cursor/rules/no-semantic-debt-workarounds.mdc`, layer dodge).
- This work merges with `docs/plans/selfhost-compiler-core-rewrite.md`
  Phase 3/4; the plan gap is tracked in #949.
