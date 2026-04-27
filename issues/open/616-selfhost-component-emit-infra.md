---
Status: open
Created: 2026-04-25
Updated: 2026-04-25
ID: 616
Track: component-model
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v2: True
# Selfhost compiler: Implement component emission infrastructure
---
# Selfhost compiler: Implement component emission infrastructure

## Summary

The `#529` 100% selfhost transition plan removed the original Rust-based component generation logic (`crates/ark-wasm/src/emit`). The selfhost compiler currently lacks the core capability to generate Wasm Components; `src/compiler/emitter.ark` only emits core Wasm modules and does not have an `emit_component` function or equivalent structural capacity. This issue tracks the core infrastructure work required to build Wasm Components natively in the Ark selfhost emitter.

## Acceptance Criteria

- [ ] Implement `emit_component` in `src/compiler/emitter.ark` (or equivalent module)capable of producing standard `.component.wasm` binaries.
- [ ] Add basic translation structures for lowering WIT interface types and core instance generation inside the component binaries.
- [ ] Verify that a simple structural component can be emitted natively without relying exclusively on `wasm-tools component new` as a system subprocess (or establish the `wasm-tools` shelling mechanism natively if preferred).

## Downstream Impact

Issue #034 (CLI WIT integration) is blocked pending this foundational infrastructure.