---
Status: done
Created: 2026-03-31
Updated: 2026-06-14
Closed: 2026-06-14
ID: 358
Track: stdlib-api
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: yes
Priority: 1
---

## Closed — 2026-06-14

`std::host::env::var` slice: removed P1 stub in `intrinsic_env_var.ark`; lookup uses
`environ_get` via `intrinsic_env_var_lookup.ark`. Fixtures: `env_var_lookup.ark`,
`env_var_default.ark`. http/sockets remain tracked via #446/#447 / host_stub gate (#292).

## Acceptance

- [x] `std::host::env::var` が環境変数を実際に読み出す実装を持つ (P1 WASI environ)
- [x] env fixtures exist in `tests/fixtures/stdlib_env/` and manifest
- Deferred: `std::host::http::*`, `std::host::sockets::connect` (see #446, #447)

# Stdlib: host family の stub を解消し stable capability に引き上げる

## Summary

`std::host::http::{get, request}`、`std::host::sockets::connect`、`std::host::env::var` など、manifest 上 `host_stub` または stable 表記だが実装が stub のままの host API を、実行可能な stable capability に引き上げる。

## References

- `src/compiler/wasm/intrinsic_env_var.ark`
- `tests/fixtures/stdlib_env/env_var_lookup.ark`
- `issues/done/633-host-capability-surface-honesty-vs-selfhost-runtime.md`
