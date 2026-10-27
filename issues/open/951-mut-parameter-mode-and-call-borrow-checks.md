---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 951
Track: typecheck
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "ADR-088 decision for #943 (2026-10-27)"
Related: "ADR-059, ADR-060, ADR-061, ADR-088, ADR-014, #936, #943, #944"
---

# 951 — implement ADR-088: `mut` parameter mode, mutable-place and same-place conflict checks

## Summary

ADR-088 decides that non-`Copy` arguments are call-scoped borrows and that
mutation across a call requires a `mut` parameter (or `mut self`) and a
mutable place at the call site. None of this is implemented: there is no
`mut` parameter syntax (`rg 'fn …\(mut x:' std src/compiler` finds 0 uses),
and today a callee can mutate an immutable caller binding through any
parameter (#943 `alias.ark` repro, observed with `wasmtime run` on both
targets printing `3` / `2`).

## Scope

1. Parser / AST: `mut` before a parameter name, `mut self`, and `|mut v|`
   closure parameters. Formatter and LSP follow (`src/compiler/parser/`,
   `src/compiler/fmt/`).
2. Typechecker, keyed on binding state (`src/compiler/typechecker/ownership.ark`,
   `call_fn.ark`, `call_method.ark`):
   - Assignment to a place rooted at a shared parameter / `self` is
     rejected (E0207 family).
   - Passing a shared-rooted or immutable-binding-rooted place to a `mut`
     parameter, or calling a `mut self` method on it, is rejected with a new
     diagnostic code.
   - The same root place passed to a `mut` parameter and another parameter
     in one call is rejected.
   - A borrowed `Box<T>` parameter cannot be returned, stored, or `unbox`ed
     (ADR-088 §6).
   - The parameter mode is part of `FnSig` and function types; a
     shared-parameter fn value is assignable to the `mut` form only.
3. stdlib: mutating operations (`push`, `set`, `pop`, `insert`, `clear`, …)
   declare `mut` receivers / parameters through their real signatures (#936),
   with no name-based exemption.
4. Migration (ADR-014): the new diagnostics start as warnings with a
   ratchet count, `std/` and `src/compiler/` are migrated (`let mut`, `mut`
   parameters), then the diagnostics become errors.
5. Docs: `docs/language/memory-model.md`, `docs/language/spec.md` §2.4 and
   `docs/current-state.md` describe ADR-088 and the current gap (#943
   acceptance).

## Acceptance

- [ ] `alias.ark` (#943) is a compile-error fixture on `wasm32` and
      `wasm32-gc`: assignment through the shared parameter of `bump` and
      `push` on the shared parameter of `add` are diagnosed.
- [ ] A positive run fixture: `fn bump(mut p: Point)` with `let mut p`
      prints the mutated value identically on both targets.
- [ ] Negative fixtures: immutable binding to a `mut` parameter, `mut self`
      method on an immutable receiver, same root place passed twice with one
      `mut`, borrowed `Box<T>` returned, `fn(T)` vs `fn(mut T)` assignment
      direction.
- [ ] Escape by sharing is covered by a run fixture whose observable output
      matches on both targets (ADR-088 §6).
- [ ] After migration the diagnostics are errors; `python3 scripts/manager.py
      verify fixtures` and `python3 scripts/manager.py selfhost fixpoint`
      stay green.
- [ ] Language docs and current-state are synced (`$docs-sync`).
