---
Status: done
Created: 2026-03-29
Updated: 2026-03-30
ID: 180
Track: main
Depends on: 179
Orchestration class: implementation-ready
---
# v5 Backend: deterministic Wasm binary emission
**Blocks v1 exit**: no

## Summary

MIR から決定的な Wasm バイナリを生成する。section writer、LEB128、target split、validation、fixpoint に効く deterministic ordering をここで追う。

## Acceptance

- [x] section writer / LEB128 / binary output の責務が明确
- [x] T1 / T3 backend split が追跡できる
- [x] validation と deterministic output requirements が完了条件に含まれている

## References

- `issues/open/179-v5-hir-to-mir-lowering.md`
- `crates/ark-wasm/src/emit/`