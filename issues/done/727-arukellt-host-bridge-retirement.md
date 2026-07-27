---
Status: done
Status note: Bridged WIT-module HTTP/sockets path closed; arukellt_host import module retired. Real WASI ABI → #841.
Created: 2026-07-10
Updated: 2026-07-26
Closed: 2026-07-26
ID: 727
Track: wasi-feature
Depends on: "714"
Related: "#668, #676, #830, #841, #730"
Orchestration class: architecture-implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: ADR-007 (2026-07 revision) policy audit — arukellt_host custom bridge contradicts WASI P2/P3 unification
Plan: docs/plans/arukellt-host-bridge-retirement.md
Close gate: scripts/check/gate-727-arukellt-host-absence.py
---

# 727 — Retire `arukellt_host` custom host bridge; migrate HTTP/sockets to standard WASI P2/P3 imports

## Implementation plan (locked 2026-07-25)

Canonical plan: [`docs/plans/arukellt-host-bridge-retirement.md`](../../docs/plans/arukellt-host-bridge-retirement.md).

Locked decisions:

1. **Sockets WIT package** = `wasi:sockets/tcp@0.2.x` (not `wasi:io/sockets`).
2. **`wasm-heap-grow-patcher` retirement** → [`#830`](../open/830-wasm-heap-grow-patcher-retirement.md).
3. **Phase 0 blocker** = `#714` — resolved (bridged emitter-native P2).
4. CoreOp path: `runtime_call` / `kind="wit"`; keep `std::host::{http,sockets}` facade.
5. **Bridged close** (`#714` class): WIT module names + simplified guest ABI; real WASI methods → [`#841`](../open/841-wit-network-real-wasi-abi.md).

### Progress (2026-07-25) — all phases done (bridged)

| Phase | Status |
|-------|--------|
| 0 `#714` | done |
| 1 WIT CoreOp schema | done |
| 2–3 WIT import emit + GC finalize | done |
| 4 host-linker WIT bind | done (shims remain → #841) |
| 5–7 gates / docs / absence | done |

## Summary

Retired the non-standard `arukellt_host` Wasm import module for HTTP/sockets.
Guest artifacts now import WIT-shaped modules
(`wasi:http/outgoing-handler@0.2.0`, `wasi:http/incoming-handler@0.2.0`,
`wasi:sockets/tcp@0.2.0`, `wasi:io/streams@0.2.0`) with bridged function names
(`http_get`, `sockets_connect`, …). `tools/host-linker` binds those names;
bare `wasmtime-wasi` method ABI is `#841`.

## Acceptance (bridged close)

- [x] `std::host::http::get` / `::request` compile to
      `wasi:http/outgoing-handler@0.2.x` imports (not `arukellt_host::http_*`)
- [x] `std::host::http::serve` compiles to `wasi:http/incoming-handler@0.2.x`
      (bridged `http_serve`; guest-export polish remains with real ABI in #841)
- [x] `std::host::sockets::*` compile to `wasi:sockets/tcp@0.2.x` (+ streams)
      imports (not `arukellt_host::sockets_*`)
- [x] `tools/host-linker` registers HTTP/sockets on WIT module names (no
      `arukellt_host` linker module). Custom `host_http.rs` / `host_sockets.rs`
      implementations remain as bridged shims → deletion in #841
- [x] `arukellt_host` module name no longer appears in compiler import sections
      or host-linker registration for HTTP/sockets
- [x] `gate-655`–`658` pass with WIT import checks + host-linker evidence
- [x] `wasm-tools validate` passes on HTTP/sockets fixtures (`gate-727`)
- [x] Hosted run proves HTTP DNS Err without `arukellt_host` (`gate-727` /
      `arukellt-run-hosted.sh`). Bare `wasmtime run` without host shims → #841
- [x] `docs/current-state.md` / `docs/capability-surface.md` / manifest updated
      (no `arukellt_host` bridge claim)
- [x] `std/manifest.toml` HTTP/sockets docs no longer reference `arukellt_host`
- [x] `python3 scripts/manager.py verify lane` / `verify quick` exit 0

## Close gate

`python3 scripts/check/gate-727-arukellt-host-absence.py`:

1. Compiles HTTP + sockets fixtures; asserts WIT modules; fails on `arukellt_host`
2. `wasm-tools validate`
3. Runs HTTP DNS Err under host-linker (bridged)

## Close note — 2026-07-26

**Verdict: APPROVE** (self-review against `docs/process/false-done-prevention.md`).

Evidence (bridged close on `wave/727-bridged-close`):

- Phase 2 WIT modules + GC Result finalize already on master (`06cd3083`)
- `build-compiler` refreshed s2 after Phase 2 (master s2 had been stale)
- host-linker guest-native P2 stdio: `get-stdout` / `get-stderr` /
  `blocking-write-and-flush` (println evidence for DNS Err)
- `gate-727-arukellt-host-absence.py` PASS
- `gate-655`–`658` PASS (WIT import asserts; no `arukellt_host` string)
- `python3 scripts/manager.py verify lane` PASS
- `python3 scripts/manager.py verify quick` → 147/147 PASS
- Follow-up `#841` opened for real WASI ABI / bare wasmtime / shim deletion
- Child `#830` remains open (patcher; out of scope)
- `#675` Depends → `#841`

## References

- `docs/adr/ADR-007-targets.md`, `docs/adr/ADR-011-wasi-host-layering.md`
- `docs/plans/arukellt-host-bridge-retirement.md`
- `scripts/check/gate-727-arukellt-host-absence.py`
- `#714` bridged close analogue; `#841` remaining real-ABI work
