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
Status note: Implementation-ready — gate_076 stdout passes; acceptance #4 (p2_fs_out.txt on disk) still open.
WASI Preview 2 の `wasi: "filesystem/types` と `wasi:filesystem/preopens` を"
---

# WASI P2 ネイティブ: wasi:filesystem ネイティブバインディング

## Summary

WASI Preview 2 の `wasi:filesystem/types` と `wasi:filesystem/preopens` を
Arukellt `std/path` と `std/fs` モジュールから P2 ネイティブで呼び出す。
resource 型 (`descriptor`) の canonical ABI ハンドリングを実装する必要がある。

## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: WASI Preview 2 filesystem capability is product-critical host functionality and is not explicitly tracked by an active open issue.

## Serial audit (2026-06-14, 5b9e5b3e)

- **Verdict**: false-close reverted — issue stays **open**
- **Gate stdout**: raw bytes `\xb9hello p2 fs\n` (`0xb9` prefix); `errors="replace"` in `e2a50c5c` masked UTF-8 decode failure
- **Gate filesystem**: `p2_fs_out.txt` **not** created — acceptance #4 unmet
- **WIP retained**: `p2_guest_fs_patch.py`, component wrap patches from parallel agents

## Serial audit (2026-06-14, 7f069f90 revert)

- **Verdict**: false-close — issue back to **open** (SERIAL audit)
- **gate_076**: rc=0 (validate + wasmtime stdout `hello p2 fs`); stdout prefix issue fixed in `7f069f90`
- **Strict filesystem**: `p2_fs_out.txt` still **not** created under `wasmtime run --dir <repo>`; fixture `fs::write_string` does not persist bytes
- **Acceptance #4**: `wasi_fs_p2.ark` ファイル読み書き確認 remains unmet for honest close
- **Code retained**: P2 fs guest patch + `p2_component_wrap` from `7f069f90` (stdout gate only)

## 受け入れ条件

1. `wasi:filesystem/types.{open-at, read-via-stream, write-via-stream, close}` を stdlib から呼ぶ
2. `descriptor` resource の resource.new / resource.drop の canonical ABI 実装
3. `std/path` の `read_file` / `write_file` が P2 ネイティブ実装を使う
4. fixture: `wasi_fs_p2.ark` でファイル読み書き確認

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md`
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-filesystem/`
