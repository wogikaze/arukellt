# WASI P2 ネイティブ: wasi:clocks / wasi:random / wasi:io バインディング

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 075
**Depends on**: 074
**Track**: wasi-feature
**Blocks v4 exit**: no

**Status note**: WASI feature — deferred to v5+. Requires WASI P2 runtime maturity.

## Summary

WASI Preview 2 の `wasi:clocks`・`wasi:random`・`wasi:io` パッケージを
Arukellt の std ライブラリから P2 ネイティブで呼び出せるようにする。
`wasi:clocks/wall-clock` / `wasi:clocks/monotonic-clock`・`wasi:random/random`・
`wasi:io/streams` (InputStream / OutputStream) を対象とする。

## 受け入れ条件

1. `std/time` が P2 モードで `wasi:clocks/wall-clock.now()` を呼ぶ
2. `std/random` が P2 モードで `wasi:random/random.get-random-bytes()` を呼ぶ  
3. `std/io` が P2 モードで `wasi:io/streams` の read/write を使う
4. P1 / P2 の両モードをコンパイル時フラグで切り替え可能

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md`
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-io/`
