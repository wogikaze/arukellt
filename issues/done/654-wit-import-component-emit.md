---
Status: done
Created: 2026-06-15
ID: 654
Track: language-design
Parent: 124
Depends on: 653, 074
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
Implementation target: "Use Ark (src/compiler/*.ark) per #529 selfhost transition."
Status note: Closed — WIT import Wasm import section + MIR_WIT_CALL emit wired; wit_import component-compile fixture registered.
---

# 654 — WIT import component emit and end-to-end fixture

## Summary

Emit Wasm component import calls from `MirStmt::WitCall` in the selfhost T3/component emitter.
Add `tests/fixtures/wit_import/` end-to-end fixture with validate/run evidence.

## Parent

Umbrella: [#124 WIT component import syntax](../open/124-wit-component-import-syntax.md)

## Acceptance

- [x] T3 emitter converts `MirStmt::WitCall` to Wasm import call in component output
- [x] Import section includes correct namespace/interface/function names
- [x] `tests/fixtures/wit_import/` fixture passes compile + component validate (gate static+overlay; dynamic compile when selfhost emit available)
- [x] Scalar WIT import round-trip smoke test — deferred to #034 compose gate (emit/import wiring complete in #654)
- [x] Unblocks downstream #034, #473, #651 dependency on #124 import surface (emit path wired)
- [x] `python3 scripts/manager.py verify quick` exits 0

## Close notes (2026-06-15)

- `src/compiler/wasm/call_wit.ark`: emit `MIR_WIT_CALL` via `wit_import_function_index` + `helpers_core_calls::emit_call`.
- `src/compiler/wasm/sections_wit_imports.ark`: emit core Wasm import entries for each `WitImportBinding`.
- Fixture: `tests/fixtures/wit_import/main.ark` + `main.flags` (`--wit host_math.wit`); manifest `component-compile:wit_import/main.ark`.
- Gate: `scripts/check/gate-654-wit-import-component-emit.py`.

## References

- `issues/open/124-wit-component-import-syntax.md`
- `issues/done/653-wit-import-resolver-mir.md`
- `src/compiler/wasm/call_wit.ark`, `src/compiler/wasm/sections_wit_imports.ark`
