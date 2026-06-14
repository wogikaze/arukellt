---
Status: open
Created: 2026-06-15
ID: 657
Track: wasi-feature
Parent: 139
Depends on: 074, 137
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Status note: Child of #139 — TCP connect + read/write client path.
---

# 657 — WASI P2 sockets: connect and read/write

## Summary

Implement `std::host::sockets` client path: TCP connect, read, write minimal API.
T1 rejection and T3 wasmtime socket I/O smoke for outbound connections.

## Parent

Umbrella: [#139 WASI P2 sockets](139-std-wasi-sockets-p2.md)

## Acceptance

- [ ] Connect/read/write surface in `std::host::sockets` + `std/manifest.toml`
- [ ] T1 `use std::host::sockets` compile-time error fixture
- [ ] P2 host call lowering for TCP connect and stream I/O
- [ ] T3 runtime smoke: connect to loopback and exchange bytes
- [ ] Doc comments and stdlib reference updated
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/139-std-wasi-sockets-p2.md`
- `docs/adr/ADR-011-wasi-host-layering.md`
- `std/host/sockets.ark`
