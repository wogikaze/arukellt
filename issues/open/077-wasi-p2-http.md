# WASI P2: `std::wasi::http` facade と runtime 検証

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-29
**ID**: 077
**Depends on**: 074, 137
**Track**: wasi-feature
**Blocks v1 exit**: no

**Status note**: WASI feature — deferred to v5+. Requires WASI P2 runtime maturity.

## Summary

WASI Preview 2 の `wasi:http/incoming-handler` と `wasi:http/outgoing-handler` を
`std::wasi::http` として提供する。
HTTP サーバ (incoming-handler world をエクスポート) と
HTTP クライアント (outgoing-handler をインポート) の両方を capability-based facade に載せる。

## 受け入れ条件

1. `std::wasi::http` に request / response / headers / body streaming の最小 API を追加する
2. T1 で `use std::wasi::http` した場合は専用 diagnostics で compile-time error になる
3. Arukellt プログラムが `wasi:http/proxy` world として HTTP サーバになれる
4. compile fixtures, runtime fixtures, docs examples が追加される
5. wasmtime (`wasi-http` feature) 上の T3 実行で HTTP client / server の両方を確認する

## 実装タスク

1. request / response / header map / body stream の public surface を設計する
2. `wasi:http` binding と host lowering を backend に追加する
3. T1 reject fixture と T3 runtime smoke test を追加する
4. doc comments から `docs/stdlib` を更新する

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §wasi:http
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-http/`
