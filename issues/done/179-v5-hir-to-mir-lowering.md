---
Status: done
Created: 2026-03-29
Updated: 2026-03-30
ID: 179
Track: main
Depends on: 164
Orchestration class: implementation-ready
---
# v5 Backend: HIR to MIR lowering
**Blocks v1 exit**: no

## Summary

typed HIR から MIR への lowering を selfhost backend の前段として実装する。旧 Phase 2/3 境界にまたがっていた責務をここに切り出す。

## Acceptance

- [x] HIR→MIR lowering rules が追跡できる
- [x] control-flow flattening と operand / type lowering の責務が整理されている
- [x] Wasm emitter が前提とする MIR surface が揃う

## References

- `issues/open/164-v5-phase2-resolver-typechecker.md`
- `issues/done/168-v5-ir-spec-doc.md`
- `crates/ark-mir/src/lower/`