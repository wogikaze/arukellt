---
Status: open
Created: 2026-06-15
ID: 656
Track: wasi-feature
Parent: 077
Depends on: 074, 137
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Status note: Child of #077 — incoming HTTP server (`wasi:http/incoming-handler`, proxy world).
---

# 656 — WASI P2 HTTP incoming server facade

## Summary

Implement `std::host::http` incoming-handler surface so Arukellt programs can export `wasi:http/proxy` world.
T1 rejection and T3 wasmtime server smoke.

## Parent

Umbrella: [#077 WASI P2 HTTP](077-wasi-p2-http.md)

## Acceptance

- [ ] Incoming-handler server API surface in `std::host::http`
- [ ] Program can export `wasi:http/proxy` world as HTTP server
- [ ] T1 reject fixture passes
- [ ] T3 runtime smoke: incoming request handling on wasmtime
- [ ] Compile + runtime fixtures under `tests/fixtures/t3/` or `stdlib_host/`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/077-wasi-p2-http.md`
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-http/`
- `std/host/http.ark`
