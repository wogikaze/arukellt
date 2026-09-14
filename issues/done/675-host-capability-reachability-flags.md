---
Status: done
Status note: Superseded by ADR-054 — the old host-capability surface was deleted instead of made user-reachable.
Created: 2026-06-17
Updated: 2026-09-13
Closed: 2026-09-13
ID: 675
Track: capability
Depends on: "841"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: P1 host capability checklist audit 2026-06-17 — superseded by ADR-054
Plan: docs/plans/675-host-capability-reachability-flags.md
---

# 675 — Host capability user-reachability and runtime permission flags

## Resolution

This issue is retired as superseded. The requested HTTP, TCP, UDP, and stream
reachability work assumed the `std::host` network modules and the Rust
`host-linker` runtime would remain available. ADR-054 chose the opposite
boundary: those legacy surfaces, their ABI aliases, and the runtime are
deleted, with official WASI Component interfaces as the only supported
replacement boundary.

The old `--allow-http`, `--deny-http`, `--allow-net`, and `--deny-net` flags
were not added because there is no longer a repository-owned network runtime
to control. HTTPS/TLS and UDP remain outside the current official interface
surface rather than being represented by a compatibility facade.

## Close evidence

- `std/host/http.ark`, `std/host/sockets.ark`, `std/host/streams.ark`, and
  `std/host/udp.ark` are deleted.
- The corresponding compiler network runtime modules, fixtures, manifest
  entries, and legacy gates are deleted.
- `tools/host-linker`, the WASI adapter/bridge, and the repository Cargo
  workspace are deleted.
- Official WASI P2 packaging is implemented by
  `scripts/run/arukellt-selfhost.sh` through `wasm-tools component embed/new`.
- Binding design and the deliberate breaking change are recorded in
  [ADR-054](../../docs/adr/ADR-054-host-linker-and-rust-runtime-retirement.md).

The historical acceptance criteria that required retaining the old network
surface are therefore not implementation requirements for the current
repository. Dynamic selfhost refresh remains tracked separately by the
bootstrap verification contract; this supersession does not claim that an
unverified stale compiler artifact is current.

## References

- [ADR-054](../../docs/adr/ADR-054-host-linker-and-rust-runtime-retirement.md)
- [#841 real WASI ABI](841-wit-network-real-wasi-abi.md)
- [historical implementation plan](../../docs/plans/675-host-capability-reachability-flags.md)
