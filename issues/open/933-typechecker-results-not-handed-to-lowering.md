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
- [ ] No lowering / emit decision reads a local's interned type-name string
      (`MirLocal_type_name()` over `type_name_id`); decisions use the TypeId /
      `MirValueType` that the checker's table supplies (aligns with ADR-053
      Phase 4 "typed-by-construction"). `MirLocal` itself already stores an
      interned id, not a `String` field (see the #949 correction).
- [ ] The repro fixtures registered by #935–#941 pass on both `wasm32` and
      `wasm32-gc`; `python3 scripts/manager.py verify fixtures` and
      `python3 scripts/manager.py selfhost fixpoint` stay green.

## Update (2026-10-30, `wave/functionid-handoff`) — first checked call-resolution table

Landed the per-call receiver-owner slice for the #936 prelude callable set,
without keeping a lowering-side fallback:

- `TypeCheckResult` exports `resolved_call_owners`, keyed by the source body
  (`decl_index` + impl-method ordinal), span, and callee. The typechecker
  records `String` / `Vec` for `len` and for the first parameter of generic
  Vec/String calls (`push`, `get`, `pop`, ...). `CoreHirResolvedCall` now
  carries this real resolution instead of the syntactic callee scan;
  `frontend_resolved_calls::scan_resolved_calls` is deleted.
- MIR lowering registers the table on `LowerCtx` and sets the body identity
  when it lowers a top-level body or impl method. `len` and `pop` bind their
  owner-qualified FunctionId (`String::len` / `Vec::len` / `Vec::pop`) from
  that table; `len` no longer reads the receiver local's type spelling, and a
  site with no resolution is not treated as the prelude free.
- Builtin `Result` match payload bindings were `TY_UNKNOWN` because
  `match_enum_info` only knows declared enums, so a `Result<T, _>::Ok(v)`
  binding lost `v`'s type and downstream calls (for example `len(keys)` after
  `table_keys()`) never reached the owner record. `match.ark` now binds
  `Ok`/`Err` payloads from the scrutinee's type arguments when no nominal
  `EnumInfo` exists. This exposed a previously masked i64/i32 comparison in
  `std/core/raw.ark`'s self-check (`9000000000` vs an `i64` payload); the
  literal now carries the `i64` suffix. `Option::Some` payload binding is
  deliberately not included yet: the same change exposes latent i32/struct
  mismatches across std sort/collection code, which belongs to the wider
  typed-locals item rather than this handoff.
- `for` range bounds were not inferred at all (`for i in 0..len(values)`
  skipped the `len` call), so those calls never reached the owner record.
  `stmt_for.ark` now infers both range bound expressions.
- An unknown-receiver method call now resolves through the only declared
  `Owner::method` signature with that name when exactly one exists; this
  types results such as `table_keys()`/`child_count()` without inventing a
  receiver dispatch the checker cannot prove.
- Impl bodies are keyed by impl decl index plus call span, without a method
  ordinal: the checker and MIR enumerate impl members differently, so an
  ordinal is not a shared identity.
- `infer_block_expr` treated a trailing statement in an `if`/`match` arm as a
  plain expression, so a final `while len(x) > 0` or `return Some(len(x))`
  never had its condition/value inferred. Trailing `while` and `return`
  statements now run through `check_stmt` before the arm's result inference.
- String method `.len()` now lowers to the owner-qualified `String::len`
  instead of the bare `len` spelling, so method-call sites do not depend on
  the free call's receiver table.
- `builtin_call_return_type` now types the `string_from_bytes` intrinsics as
  `String`, so `len(__intrinsic_string_from_bytes(bytes))` reaches the owner
  record instead of lowering as an unresolved free `len`.
- Fixtures `stdlib_vec/prelude_call_owner_untracked_receiver{,_wasm32}.ark`
  cover a receiver (if-expression result) whose MIR local loses its element
  type.

This is one table, not the full item: expression/binding TypeIds, FunctionId
for every call, and deletion of the remaining MIR re-inference passes
(`method_resolve`, `call_type_fallback*`, `post_pass_type_propagate`,
`post_pass_callee_lookup`) are unchanged. `push` / `get` still lower through
`call_rewrite_vec*` element-type predicates and #949's plan gap remains.

## Notes

- No bridge that re-derives types "where the table is empty" may land
  (`.cursor/rules/no-semantic-debt-workarounds.mdc`, layer dodge).
- This work merges with `docs/plans/selfhost-compiler-core-rewrite.md`
  Phase 3/4; the plan gap is tracked in #949.
