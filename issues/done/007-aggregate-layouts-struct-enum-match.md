---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 7
Track: main
Depends on: 004, 006
Orchestration class: implementation-ready
---
# T3 aggregate layouts: struct enum match
**Blocks v1 exit**: yes

## Summary

Stabilize GC-native layout and match behavior for user structs, enums, Option/Result, and aggregate field access in T3.

## Acceptance Criteria

- [x] Struct initialization and field access compile/run correctly under T3.
- [x] Enum construction, payload access, and match lowering work under T3 for both builtin and user-defined enums.
- [x] Option/Result behavior in T3 is consistent with the frontend type system and MIR lowering rules.
- [x] Layout comments, type-table usage, and docs all describe the same T3 aggregate model.

## Goal

Make aggregate data a first-class GC-native part of T3 rather than a partially boxed/bridged special case.

## Implementation

- Fix and document the layout rules for user structs and enums in `crates/ark-wasm/src/emit/t3_wasm_gc.rs`.
- Align enum tag/payload lowering with whatever MIR currently guarantees, and stop relying on ad hoc fallback assumptions when possible.
- Verify builtin enums (`Option`, `Result`) and user enums share a coherent backend model.
- Ensure match lowering and emitter decoding rules are consistent for tuple variants, struct variants, and tag-only variants.

## Dependencies

- Issues 004 and 006.

## Impact

- T3 backend
- MIR type-table consumers
- enum/struct fixtures

## Tests

- Struct field tests.
- Enum construction/match tests.
- Option/Result tests.
- Nested aggregate cases.

## Docs updates

- `docs/language/type-system.md`
- `docs/language/error-handling.md`
- `docs/platform/abi.md`

## Compatibility

- T3 aggregate representation changes.
- Source-level semantics remain unchanged.

## Notes

- Keep frontend exhaustiveness semantics unchanged; this issue is about backend layout fidelity, not frontend type rules.