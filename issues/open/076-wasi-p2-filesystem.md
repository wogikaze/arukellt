---
Status: open
Created: 2026-03-28
Updated: 2026-06-14
ID: 62
Track: wasi-feature
Depends on: 074, 510
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
Status note: Implementation-ready — upstream gates #074 and #510 resolved.
WASI Preview 2 の `wasi: "filesystem/types` と `wasi:filesystem/preopens` を"
---

# WASI P2 ネイティブ: "wasi:filesystem ネイティブバインディング"

1. `wasi: filesystem/types.{open-at, read-via-stream, write-via-stream, close}` を stdlib から呼ぶ
2. fixture: `wasi_fs_p2.ark` でファイル読み書き確認

# WASI P2 ネイティブ: wasi:filesystem ネイティブバインディング

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

## Serial audit (2026-06-14)

- **Verdict**: false-close reverted — issue stays **open**
- **Gate stdout**: raw bytes `\xb9hello p2 fs\n` (`0xb9` prefix); `errors="replace"` in `e2a50c5c` masked UTF-8 decode failure
- **Gate filesystem**: `p2_fs_out.txt` **not** created — stub adapter false-success (acceptance #4 unmet)
- **Removed**: `errors="replace"` decode workaround in close-gate runner
- **WIP retained**: `p2_guest_fs_patch.py`, flattened WIT shims, compiler P2 import plumbing from `5a10b96b`

## 受け入れ条件

1. `wasi:filesystem/types.{open-at, read-via-stream, write-via-stream, close}` を stdlib から呼ぶ
2. `descriptor` resource の resource.new / resource.drop の canonical ABI 実装
3. `std/path` の `read_file` / `write_file` が P2 ネイティブ実装を使う
4. fixture: `wasi_fs_p2.ark` でファイル読み書き確認

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md`
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-filesystem/`
