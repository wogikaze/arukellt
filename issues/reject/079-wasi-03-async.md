# WASI 0.3-rc: async func / stream<T> / future<T> コンパイルサポート

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 079
**Depends on**: 074
**Track**: wasi-feature
**Blocks v4 exit**: no

**Status note**: WASI feature — deferred to v5+. Requires WASI P2 runtime maturity.


## Audit classification

- **Date**: 2026-04-21
- **Classification**: superseded-by-existing-open
- **Reason**: Superseded by active open issue #474 (async component support), which tracks the broader async WIT / async WASI P2 product surface.

## Summary

WASI 0.3.0-rc (`docs/spec/spec-WASI-0.3.0-rc/OVERVIEW.md`) で導入される
`async func`・`stream<T>`・`future<T>` 型を Arukellt がコンパイル対象として扱えるようにする。
WASI 0.3 は CM (Component Model) に組み込まれた非同期プリミティブを持ち、
`poll_oneoff` 相当のポーリングループが不要になる。

## 背景

WASI 0.3 では `wasi:http/handler` の `handle` が `async func` になり、
`wasi:io` の `input-stream.read()` が `future<list<u8>>` を返す。
Arukellt で async/await 構文がない場合でも、
「WASI 0.3 の async ABI に従ったコンポーネントをエクスポートする」コードを生成できれば十分。

## 受け入れ条件

1. `--wasi-version 0.3` フラグで WASI 0.3 RC コンポーネント生成
2. `stream<T>` / `future<T>` を Arukellt の `Stream<T>` / `Future<T>` 型にマッピング
3. `wasi:http/service` world のエクスポートが WASI 0.3 canonical ABI に従う
4. wasmtime WASI 0.3 対応版で動作確認 (RC 段階のため条件付き)

## 注意

WASI 0.3.0-rc は 2026-03 時点で RC 段階。API が変わる可能性があるため、
実装は `#[cfg(feature = "wasi-03")]` フラグで隔離すること。

## 参照

- `docs/spec/spec-WASI-0.3.0-rc/OVERVIEW.md`
