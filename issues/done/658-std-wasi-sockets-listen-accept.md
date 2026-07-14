---
Status: done
Created: 2026-06-15
Closed: 2026-06-15
ID: 658
Track: wasi-feature
Parent: 139
Depends on: 137
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Status note: Closed — T3 listen/accept via host-linker with loopback helper client fixture.
---

# 658 — WASI P2 sockets: listen and accept

## Summary

Implement `std::host::sockets` server path: TCP listen, accept, and accepted-stream I/O.
T3 wasmtime smoke for inbound connections.

## Parent

Umbrella: [#139 WASI P2 sockets](../done/139-std-wasi-sockets-p2.md)

## Acceptance

- [x] Listen/accept surface in `std::host::sockets` + `std/manifest.toml`
- [x] P2 host call lowering for listen/accept (`import_indices`, intrinsics, host-linker)
- [x] T3 runtime smoke: bind loopback, accept client (`tests/fixtures/host/sockets/listen_accept.ark`, `gate-658-sockets-listen-accept.py`)
- [x] Compile + runtime fixtures added
- [x] Independent of #657 (parallel dispatch OK) but shares manifest/docs
- [x] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/done/139-std-wasi-sockets-p2.md`
- `issues/open/657-std-wasi-sockets-connect-read.md`
- `std/host/sockets.ark`
