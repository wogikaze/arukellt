---
Status: done
Created: 2026-03-28
Updated: 2026-06-15
Closed: 2026-06-15
ID: 63
Depends on: 074, 137
Track: wasi-feature
Orchestration class: done
Orchestration upstream: None
Blocks: none
Blocks v{N}: none
Status note: Umbrella closed after child slices #655 (outgoing client) and #656 (incoming server).
---

## Close note — 2026-06-15

WASI Preview 2 HTTP capability is available via `std::host::http` with T1 rejection and
T3 runtime smoke on wasmtime / host-linker:

- **#655** — outgoing HTTP client (`gate-655-http-outgoing.py`).
- **#656** — incoming HTTP server / `serve` surface (`gate-656-http-incoming.py`).

**Verification gate:** `scripts/check/gate-077-wasi-p2-http-umbrella.py`

---

# WASI P2: `std::host::http` facade と runtime 検証

## Summary

WASI Preview 2 の `wasi:http/incoming-handler` と `wasi:http/outgoing-handler` を
`std::host::http` として提供する。

## 受け入れ条件

1. [x] `std::host::http` に request / response / headers / body streaming の最小 API
2. [x] T1 で `use std::host::http` した場合は専用 diagnostics で compile-time error
3. [x] Arukellt プログラムが `wasi:http/proxy` world として HTTP サーバになれる
4. [x] compile fixtures, runtime fixtures, docs examples が追加される
5. [x] wasmtime (`wasi-http` feature) 上の T3 実行で HTTP client / server を確認

## 子 issue

- [#655 WASI P2 HTTP outgoing client facade](655-wasi-p2-http-outgoing-client.md)
- [#656 WASI P2 HTTP incoming server facade](656-wasi-p2-http-incoming-server.md)

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §wasi:http
