---
Status: done
Created: 2026-03-29
Updated: 2026-03-29
ID: 187
Track: parallel
Depends on: 199, 200, 201
Orchestration class: implementation-ready
Blocks v1 exit: False
Status note: Parent issue for debug metadata, runtime inspection, and advanced debug intelligence.
# Debug UX: source-level debugging と DAP surface
---
# Debug UX: source-level debugging と DAP surface


## Summary

Arukellt の debug UX は、metadata / DAP foundation、通常の inspect/step/evaluate surface、高度な履歴・可視化 DX で分けて扱う必要がある。
最小 DAP 実装と stretch DX を同じ本文に混ぜず、段階的に追えるよう child issue に分離する。

## Acceptance

- [x] #199, #200, #201 が完了している
- [x] DAP foundation / runtime inspection / advanced debug intelligence の責務が child issue に分解されている
- [x] debug UX の残課題が issue queue 上で追跡できる

## References

- `issues/open/199-debug-metadata-and-dap-adapter-foundation.md`
- `issues/open/200-runtime-inspection-stepping-and-evaluate-surface.md`
- `issues/open/201-advanced-debug-intelligence.md`
- `docs/current-state.md`
- `crates/arukellt/src/main.rs`