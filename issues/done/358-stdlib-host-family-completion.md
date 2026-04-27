---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 358
Track: stdlib-api
Depends on: —
Orchestration class: implementation-ready
---
# Stdlib: host family の stub を解消し stable capability に引き上げる
**Blocks v1 exit**: yes
**Priority**: 1

## Summary

`std::host::http::{get, request}`、`std::host::sockets::connect`、`std::host::env::var` など、manifest 上 `host_stub` または stable 表記だが実装が stub のままの host API を、実行可能な stable capability に引き上げる。host family は利用者が最も直接依存する API 群であり、stub のまま公開するのは product として不整合。

## Current state

- `std/manifest.toml`: `host_stub` が 3 関数 (http::get, http::request, sockets::connect)
- `std::host::env::var` は stability=stable だが実装は stub で環境変数 lookup が未実体化
- host family は WASI P2 capability に依存し、target=wasm32-wasi-p2 のものが多い
- `docs/current-state.md` は active work を WASI / `std::host::*` rollout に設定している

## Acceptance

- [x] `std::host::http::get` と `std::host::http::request` が WASI P2 target で実行可能になる
- [x] `std::host::sockets::connect` が WASI P2 target で実行可能になる
- [x] `std::host::env::var` が環境変数を実際に読み出す実装を持つ
- [x] 各関数に対応する fixture テストが `tests/fixtures/` に存在し pass する
- [x] `std/manifest.toml` の kind が `host_stub` から適切な kind に更新される

## References

- `std/manifest.toml` — host_stub 関数定義
- `std/host/**/*.ark` — host module 実装
- `crates/ark-wasm/src/emit/t3/` — WASI import 生成
- `docs/current-state.md` — active work 記述