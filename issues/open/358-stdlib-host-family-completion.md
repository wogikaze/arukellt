---
Status: open
Created: 2026-03-31
Updated: 2026-06-12
ID: 358
Track: stdlib-api
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: yes
Priority: 1
---

# Stdlib: host family の stub を解消し stable capability に引き上げる

## Reopened by audit — 2026-06-12 (slice D)

**Classification**: `must-reopen` / `acceptance-not-actually-met`

**Reopen reason**: Close acceptance claims http, sockets, and `env::var` are executable stable capabilities with passing fixtures. On the sole selfhost execution path, none of the three surfaces is backed end-to-end.

**Repo evidence**:

- `src/compiler/wasm/call_host_io.ark` dispatches only `env` argv helpers, `fs`, `process::exit`, and `stdio` — no `__intrinsic_http_*`, `__intrinsic_sockets_*`, or real `env::var` lookup.
- `src/compiler/wasm/intrinsic_env_args.ark` `emit_env_var` / `emit_env_get_var` call `emit_env_missing_var_option` (always `None`).
- `std/manifest.toml` advertises `std::host::http` and `std::host::sockets` with `availability.t1/t3=true` referencing deleted `arukellt_host` linker symbols.
- Sub-issues #446 and #447 were reopened in wave 3 for http/sockets; this umbrella issue still listed all acceptance items checked.

**Violated acceptance**: all five checkboxes below (http, sockets, env::var, fixture pass claims, host_stub kind migration).

**Evidence files**: `src/compiler/wasm/call_host_io.ark`, `src/compiler/wasm/intrinsic_env_args.ark`, `std/manifest.toml`, `std/host/http.ark`, `std/host/sockets.ark`, `std/host/env.ark`, `issues/open/446-std-host-http-implementation.md`, `issues/open/447-std-host-sockets-implementation.md`, `issues/open/633-host-capability-surface-honesty-vs-selfhost-runtime.md`

**Follow-up split**: none (track via #446, #447, #633, #051)

---

`std: ":host::http::{get, request}`、`std::host::sockets::connect`、`std::host::env::var` など、manifest 上 `host_stub` または stable 表記だが実装が stub のままの host API を、実行可能な stable capability に引き上げる。host family は利用者が最も直接依存する API 群であり、stub のまま公開するのは product として不整合。"
- `std/manifest.toml`: "`host_stub` が 3 関数 (http::get, http::request, sockets::connect)"
- `std: ":host::env::var` は stability=stable だが実装は stub で環境変数 lookup が未実体化"
- `docs/current-state.md` は active work を WASI / `std: ":host::*` rollout に設定している"
- [x] `std: ":host::env::var` が環境変数を実際に読み出す実装を持つ"

# Stdlib: host family の stub を解消し stable capability に引き上げる

## Summary

`std::host::http::{get, request}`、`std::host::sockets::connect`、`std::host::env::var` など、manifest 上 `host_stub` または stable 表記だが実装が stub のままの host API を、実行可能な stable capability に引き上げる。host family は利用者が最も直接依存する API 群であり、stub のまま公開するのは product として不整合。

## Current state

- `std/manifest.toml`: `host_stub` が 3 関数 (http::get, http::request, sockets::connect)
- `std::host::env::var` は stability=stable だが実装は stub で環境変数 lookup が未実体化
- host family は WASI P2 capability に依存し、target=wasm32-wasi-p2 のものが多い
- `docs/current-state.md` は active work を WASI / `std::host::*` rollout に設定している

## Acceptance

- [ ] `std::host::http::get` と `std::host::http::request` が WASI P2 target で実行可能になる
- [ ] `std::host::sockets::connect` が WASI P2 target で実行可能になる
- [ ] `std::host::env::var` が環境変数を実際に読み出す実装を持つ
- [ ] 各関数に対応する fixture テストが `tests/fixtures/` に存在し pass する
- [ ] `std/manifest.toml` の kind が `host_stub` から適切な kind に更新される

## References

- `std/manifest.toml` — host_stub 関数定義
- `std/host/**/*.ark` — host module 実装
- `crates/ark-wasm/src/emit/t3/` — WASI import 生成
- `docs/current-state.md` — active work 記述
