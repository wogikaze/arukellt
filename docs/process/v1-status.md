# V1 Completion Report

> **Historical report**: this page records the v1 exit state as it stood on 2026-03-27.
> It is not the current feature matrix. For current behavior, use [`../current-state.md`](../current-state.md).

## Status: **V1 EXIT CRITERIA MET — GC-NATIVE**

All v1 exit criteria were verified and satisfied when the v1 track closed.
GC-native codegen (T3 emitter) was complete as of 2026-03-27.

## Verification Results (at v1 close)

- verify harness passed
- unit tests passed (workspace, excluding ark-llvm)
- fixture harness passed for the v1 exit set
- clippy and formatting were clean

## What v1 established

- T3 (`wasm32-wasi-p2`) compile/run correctness on the v1 scope
- GC-native T3 data model
- T1 retained as compatibility path
- `RuntimeModel::T3WasmGcP2` as the active T3 runtime model

## Historical scope boundaries

At the moment v1 closed:

- Component output was still outside the v1 exit gate
- native / LLVM remained scaffold-only
- broader post-v1 work was deferred

That boundary is historical. Later tracks added component / WIT support on top of the v1 base.

## Source of truth hierarchy

- historical v1 exit report: this page
- current behavior: `docs/current-state.md`
- design rationale: `docs/adr/`
- current operational policy: `docs/process/policy.md`
