---
Status: open
Created: 2026-06-15
ID: 654
Track: language-design
Parent: 124
Depends on: 653, 074
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
Implementation target: "Use Ark (src/compiler/*.ark) per #529 selfhost transition."
Status note: Child of #124 — component/T3 emitter and end-to-end fixture slice.
---

# 654 — WIT import component emit and end-to-end fixture

## Summary

Emit Wasm component import calls from `MirStmt::WitCall` in the selfhost T3/component emitter.
Add `tests/fixtures/wit_import/` end-to-end fixture with validate/run evidence.

## Parent

Umbrella: [#124 WIT component import syntax](124-wit-component-import-syntax.md)

## Acceptance

- [ ] T3 emitter converts `MirStmt::WitCall` to Wasm import call in component output
- [ ] Import section includes correct namespace/interface/function names
- [ ] `tests/fixtures/wit_import/` fixture passes compile + component validate
- [ ] Scalar WIT import round-trip smoke test (wasmtime or compose gate)
- [ ] Unblocks downstream #034, #473, #651 dependency on #124 import surface
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/124-wit-component-import-syntax.md`
- `issues/open/653-wit-import-resolver-mir.md`
- `src/compiler/emitter.ark`, `src/compiler/component_emitter.ark`
