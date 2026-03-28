# WASI P2 ネイティブ: wasi:filesystem ネイティブバインディング

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 076
**Depends on**: 074
**Track**: wasi-feature
**Blocks v4 exit**: no

## Summary

WASI Preview 2 の `wasi:filesystem/types` と `wasi:filesystem/preopens` を
Arukellt `std/path` と `std/fs` モジュールから P2 ネイティブで呼び出す。
resource 型 (`descriptor`) の canonical ABI ハンドリングを実装する必要がある。

## 受け入れ条件

1. `wasi:filesystem/types.{open-at, read-via-stream, write-via-stream, close}` を stdlib から呼ぶ
2. `descriptor` resource の resource.new / resource.drop の canonical ABI 実装
3. `std/path` の `read_file` / `write_file` が P2 ネイティブ実装を使う
4. fixture: `wasi_fs_p2.ark` でファイル読み書き確認

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md`
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-filesystem/`
