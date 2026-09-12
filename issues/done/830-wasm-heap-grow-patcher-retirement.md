---
Status: done
Status note: Completed by deleting the heap patcher and its Rust/Cargo dependency as part of ADR-054.
Created: 2026-07-25
Updated: 2026-09-13
Closed: 2026-09-13
ID: 830
Track: selfhost-infra
Parent: 727
Depends on: "730"
Related: "#727, #686, #813"
Orchestration class: architecture-implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Split from #727 Related section — completed by direct selfhost runtime retirement
---

# 830 — Retire `wasm-heap-grow-patcher` (walrus) from selfhost bootstrap

## Resolution

The patcher is retired. The repository no longer builds or invokes a Rust
`walrus` post-processor for bootstrap Wasm. Memory sizing is part of the
compiler/runtime contract, and duplicate-export normalization is handled by
the source-side emitter and the verification tools that inspect the produced
module.

## Close evidence

- `scripts/bootstrap/wasm-heap-grow-patcher/` is deleted.
- The repository `Cargo.toml`, `Cargo.lock`, Rust toolchain file, and tracked
  Rust sources are deleted.
- The patcher's CI/build entry points and the dead binary MIR-prune patch are
  deleted from `scripts/selfhost/checks.py` and the workflow files.
- The remaining selfhost path executes Wasm directly with Wasmtime; it does
  not post-process the module through a repository-owned Rust tool.

This issue's old root-cause checklist is historical. Its requested removal is
complete, while the ordinary selfhost refresh/fixpoint evidence remains
subject to the current bootstrap verification contract.

## References

- [ADR-054](../../docs/adr/ADR-054-host-linker-and-rust-runtime-retirement.md)
- [#727 host bridge retirement](727-arukellt-host-bridge-retirement.md)
- [#730 bootstrap memory contract](730-bootstrap-wasm-4gb-memory-limit.md)
