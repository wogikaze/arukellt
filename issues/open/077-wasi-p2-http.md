# WASI P2: wasi:http IncomingHandler / OutgoingHandler 対応

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 077
**Depends on**: 074
**Track**: wasi-feature
**Blocks v4 exit**: no

## Summary

WASI Preview 2 の `wasi:http/incoming-handler` と `wasi:http/outgoing-handler` を
Arukellt の std ライブラリとして提供する。
HTTP サーバ (incoming-handler world をエクスポート) と
HTTP クライアント (outgoing-handler をインポート) の両方をサポート。

## 受け入れ条件

1. `std/http` モジュールに `send_request(method, url, headers, body)` 関数追加
2. Arukellt プログラムが `wasi:http/proxy` world として HTTP サーバになれる
3. `incoming-request` resource の WIT binding が自動生成される
4. wasmtime (wasi-http feature) で動作確認

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §wasi:http
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-http/`
