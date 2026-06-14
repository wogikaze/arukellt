---
Status: open
Created: 2026-06-15
ID: 658
Track: wasi-feature
Parent: 139
Depends on: 074, 137
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Status note: Child of #139 — TCP listen + accept server path.
---

# 658 — WASI P2 sockets: listen and accept

## Summary

Implement `std::host::sockets` server path: TCP listen, accept, and accepted-stream I/O.
T3 wasmtime smoke for inbound connections.

## Parent

Umbrella: [#139 WASI P2 sockets](139-std-wasi-sockets-p2.md)

## Acceptance

- [ ] Listen/accept surface in `std::host::sockets`
- [ ] P2 host call lowering for listen/accept
- [ ] T3 runtime smoke: bind loopback, accept client, read/write
- [ ] Compile + runtime fixtures added
- [ ] Independent of #657 (parallel dispatch OK) but shares manifest/docs
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/139-std-wasi-sockets-p2.md`
- `issues/open/657-std-wasi-sockets-connect-read.md`
- `std/host/sockets.ark`
