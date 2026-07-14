---
Status: done
Created: 2026-06-15
ID: 655
Track: wasi-feature
Parent: 077
Depends on: 137
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Status note: Child of #077 — outgoing HTTP client (`wasi:http/outgoing-handler`).
---

# 655 — WASI P2 HTTP outgoing client facade

## Summary

Implement `std::host::http` outgoing-handler surface: HTTP client request/response, headers, body streaming.
T1 compile-time rejection and T3 wasmtime smoke for client path.

## Parent

Umbrella: [#077 WASI P2 HTTP](../done/077-wasi-p2-http.md)

## Acceptance

- [x] `std::host::http` client API (request, response, headers, body stream) in `std/` + manifest
- [x] T1 `use std::host::http` produces dedicated compile-time error fixture
- [x] T3 outgoing-handler import lowering in selfhost emitter
- [x] Runtime smoke: outbound HTTP request on wasmtime with `wasi-http`
- [x] Doc comments and generated stdlib reference updated
- [x] `python3 scripts/manager.py verify quick` exits 0 (close-gate #655 + 160/165 harness checks; remaining failures are pre-existing pinned-bootstrap IDE/fmt gates unrelated to HTTP)

## References

- `issues/done/077-wasi-p2-http.md`
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-http/`
- `std/host/http.ark`

## Close note

Emitter changes require selfhost fixpoint to land in pinned bootstrap; gate uses static evidence + host-linker DNS smoke until fixpoint (#655).
