---
Status: done
Created: 2026-06-15
ID: 657
Track: wasi-feature
Parent: 139
Depends on: 074, 137
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Status note: Closed — T3 connect/read/write via host-linker with loopback echo fixture.
---

# 657 — WASI P2 sockets: connect and read/write

## Summary

Implement `std::host::sockets` client path: TCP connect, read, write minimal API.
T1 rejection and T3 wasmtime socket I/O smoke for outbound connections.

## Parent

Umbrella: [#139 WASI P2 sockets](../open/139-std-wasi-sockets-p2.md)

## Acceptance

- [x] Connect/read/write surface in `std::host::sockets` + `std/manifest.toml`
- [x] T1 `use std::host::sockets` compile-time error fixture (`tests/fixtures/target_gating/t1_import_sockets.ark`)
- [x] P2 host call lowering for TCP connect and stream I/O (`import_indices`, intrinsics, host-linker)
- [x] T3 runtime smoke: connect to loopback and exchange bytes (`tests/fixtures/host/sockets/connect_read_write.ark`, `gate-657-sockets-connect-read-write.py`)
- [x] Doc comments and stdlib reference updated
- [x] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/139-std-wasi-sockets-p2.md`
- `docs/adr/ADR-011-wasi-host-layering.md`
- `std/host/sockets.ark`
