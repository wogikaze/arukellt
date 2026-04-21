# WASI P2 ネイティブ: wasi:filesystem ネイティブバインディング

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 076
**Depends on**: 074, 510
**Track**: wasi-feature
**Blocks v4 exit**: no

**Status note**: WASI feature — deferred to v5+. Requires WASI P2 runtime maturity.

## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: WASI Preview 2 filesystem capability is product-critical host functionality and is not explicitly tracked by an active open issue. Existing issues (#074, #510, #524) only cover native component plumbing, import-table switching, or a narrow fs semantics/doc slice.
- **Audit evidence**:
  - No dedicated active open issue tracked this product gap.
  - The capability is required for the WASI P2 / component product surface, not merely future speculation.
  - Reject placement was inconsistent with current product direction.

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
