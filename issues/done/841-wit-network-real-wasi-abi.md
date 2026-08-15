---
Status: done
Created: 2026-07-26
Updated: 2026-08-14
ID: 841
Track: wasi-feature
Parent: 727
Depends on: "727"
Related: "#675, #668, #830, #714"
Orchestration class: architecture-implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Split from #727 bridged close — real WASI HTTP/sockets ABI remains after WIT module rename
---

# 841 — Lower HTTP/sockets guest ABI to real WASI methods; delete host bridge shims

## Summary

`#727` closed the **bridged** path (same class as `#714` stdio): guest core imports
WIT-shaped module names (`wasi:http/...`, `wasi:sockets/tcp@...`) with simplified
function names (`http_get`, `sockets_connect`, …). `tools/host-linker` still
implements those names in `host_http.rs` / `host_sockets.rs`.

This issue finishes the portable path:

1. Guest emits real WASI 0.2 method names / resource handles
   (e.g. `outgoing-handler.handle`, TCP create/start-connect, `wasi:io/streams`
   read/write).
2. Component / core artifacts run under bare `wasmtime run` (with standard
   `wasmtime-wasi` / `wasi-http`) without `arukellt-host-run`.
3. Delete or reduce `host_http.rs` / `host_sockets.rs` to thin unused shims,
   then remove them.
4. Rename legacy compiler flag `needs_arukellt_host` → a network/WIT-shaped
   name once bootstrap overlay matchers are updated.

## Non-goals

- Changing `std::host::{http,sockets}` user-facing facade shapes (ADR-011).
- HTTPS/TLS, HTTP/2.
- UDP (`#675`).
- `wasm-heap-grow-patcher` (`#830`).

## Acceptance

- [x] Guest HTTP/sockets call sites use real WASI method imports (not
      `http_get` / `sockets_*` bridge names), or a documented component
      canon-lower that hides only at the component boundary
- [x] `wasmtime run` executes HTTP DNS + sockets fixtures without
      `arukellt-host-run` / custom `host_http` / `host_sockets` registration
- [x] `tools/host-linker/src/host_http.rs` and `host_sockets.rs` deleted
      (or empty stubs with removal date + gate)
- [x] Compiler flag `needs_arukellt_host` renamed away from `arukellt_host`
- [x] `gate-655`–`658` / `#727` absence gate updated for the real-ABI path
- [x] `python3 scripts/manager.py verify quick` exits 0

## Close gate

Extend or replace `scripts/check/gate-727-arukellt-host-absence.py` (or add
`gate-831-*.py`) so bare wasmtime run is required and host shim files are
absent.

## References

- `issues/done/727-arukellt-host-bridge-retirement.md` (bridged close)
- `docs/plans/arukellt-host-bridge-retirement.md`
- `docs/research/p2-bridged-wasi-path-roadmap.md`
- `tools/host-linker/src/host_http.rs`, `host_sockets.rs`
- `#714` bridged close → `#668` guest-native remaining work (stdio analogue)

## Close receipt — 2026-08-14

- Dedicated close gate: PASS
- `python3 scripts/manager.py verify quick`: PASS in PR #46 CI
- Verification harness / docs consistency / selfhost gates: PASS in PR #46 CI
- Implementation PR: #46 (`feat(wasi): productionize runtime ABI and real WASI host paths`)

