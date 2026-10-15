---
Status: open
Created: 2026-10-15
Updated: 2026-10-15
ID: 924
Track: compiler
Depends on: ""
Related: "#922, #851"
Orchestration class: implementation
Blocks v4 exit: False
---

# wasm32-linear: bare tuple literal in tail position emits drop+unreachable

## Summary

On the wasm32 (linear-memory) backend, a function whose body ends in a
bare tuple literal `(a, b)` — rather than `return (a, b)` — is
miscompiled: the element values are dropped, `unreachable` is emitted,
and the real tuple construction is emitted as dead code.  Depending on
shape the result is either a runtime trap or *invalid* wasm (operand
stack underflow at validation).  `return (a, b)` emits correctly.

Found while repairing the AST cache codec (#825): every codec reader
returned tuples via bare tails, which is why the cache never worked on
the linear target.

## Evidence

- Probe (deleted after use; rebuild from git history of
  src/compiler/astc_probe_825.ark): `fn read_i32(data,pos,limit) -> (i32,i32)`
  ending in `(word, next)` → emitted `local.get word; drop; drop;
  unreachable` + dead `local.get _t57; local.set next` — the `let next =
  pos + 4` RHS was emitted *after* the terminator (scratch-local def/use
  inversion under the shadow-stack spill pass, locals 69+).
- Reproduces identically under the pinned bootstrap compiler — a
  pre-existing emitter defect, not an s2 regression.
- Runtime symptom in the live probe: wasmtime trap
  `wasm unreachable instruction executed` inside `read_i32` ←
  `deserialize_ast_nodes` ← `read_ast_cache`.

## Hypothesis

The linear backend's value tracking treats a tail-position tuple literal
as already-returned (or mis-lowers it): elements are evaluated then
dropped, a terminator is inserted, and dependent lets/temporaries are
scheduled into unreachable code.  Same family as the #922 mid-expression
ref-loss signature — worth auditing whether other tail-position
aggregate literals (record literals? multi-element returns) hit the same
path.

## Acceptance

- [ ] Bare tuple tails `(a, b)` produce correct tuple construction on
      wasm32 (parity with wasm32-gc).
- [ ] Regression fixture: a tuple-returning fn with bare tail on wasm32
      (compile + run + validate).
- [ ] Audit sibling tail-position aggregate exprs (record literal tail,
      nested tuple) for the same defect.
- [ ] If the fix lands in shared MIR→emit code, confirm wasm32-gc
      unaffected by fixture parity.

## Notes

- Workaround used in #825: always `return (a, b)` explicitly.  Until this
  is fixed, bare tuple tails on wasm32-linear silently trap or fail
  validation — consider a lint/compile-error until repaired.
