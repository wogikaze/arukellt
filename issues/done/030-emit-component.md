---
Status: done
Created: 2026-03-28
Updated: 2026-03-30
ID: 24
Track: component-model
Depends on: 029
Orchestration class: implementation-ready
---
# Enable --emit component and produce .component.wasm
**Blocks v1 exit**: no

## Summary

Remove the hard error for `--emit component` in `build_backend_plan()`. Implement
component wrapping: take the GC-native core Wasm module produced by T3, wrap it with
a component type section and canonical ABI adapters, and output a valid `.component.wasm`.

## Context

Currently `crates/ark-target/src/plan.rs:68-72` returns a hard error when
`emit_kind == EmitKind::Component`. The test `component_emit_is_rejected` in
`crates/ark-wasm/src/emit/mod.rs:88-90` explicitly asserts this rejection.

This issue removes that block and implements the actual component wrapping pipeline.

### Decision: In-tree vs external tooling

Use `wasm-tools component new` as an external subprocess for v2. Rationale:
- The component binary format is complex (component sections, canonical options, type interning).
- `wasm-tools` is the reference implementation maintained by the Bytecode Alliance.
- In-tree implementation would duplicate significant effort with low benefit for v2.
- Record this decision as ADR-008.

If `wasm-tools` is unavailable at compile time, fall back to emitting the core module
with a clear error message suggesting installation.

## Acceptance Criteria

- [x] `build_backend_plan(TargetId::Wasm32WasiP2, EmitKind::Component)` returns `Ok(plan)`
      instead of `Err(...)`.
- [x] `EmitCapability::Component` variant added to `crates/ark-target/src/plan.rs`.
- [x] `BackendPlan` for component emit includes: canonical ABI adapter requirements,
      import/export interface names, WIT type definitions.
- [x] `crates/ark-wasm/src/emit/mod.rs` `emit_with_plan()` handles
      `EmitCapability::Component`: emits core module → generates WIT → invokes component
      wrapping → validates result.
- [x] `Session::compile_component()` method added to `crates/ark-driver/src/session.rs`
      that produces `Vec<u8>` containing a valid component binary.
- [x] Component output passes `wasmparser` validation with component model features enabled.
- [x] The test `component_emit_is_rejected` is replaced by `component_emit_produces_valid_component`.
- [x] `arukellt compile --emit component hello.ark --target wasm32-wasi-p2` produces
      `hello.component.wasm` that can be inspected with `wasm-tools component wit`.
- [x] ADR-008 (Component wrapping strategy) is written to `docs/adr/ADR-008-component-wrapping.md`.

## Key Files

- `crates/ark-target/src/plan.rs` — remove hard error, add `EmitCapability::Component`
- `crates/ark-wasm/src/emit/mod.rs` — component emit routing
- `crates/ark-wasm/src/component/mod.rs` — component wrapping orchestration
- `crates/ark-driver/src/session.rs` — `compile_component()` method
- `crates/arukellt/src/main.rs` — wire `--emit component` to `compile_component()`
- `docs/adr/ADR-008-component-wrapping.md` — new ADR

## Notes

- The `--emit all` flag should also be unblocked once component emit works. It produces
  both `foo.wasm` (core) and `foo.component.wasm` (component).
- Component validation requires `wasmparser` features for component model. Check if the
  current `wasmparser` dependency version supports this; upgrade if needed.
- The component binary must embed the WIT world inline (not reference an external `.wit` file)
  so that consumers can extract it via `wasm-tools component wit`.