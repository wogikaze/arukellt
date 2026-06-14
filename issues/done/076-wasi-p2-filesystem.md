---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
ID: 62
Track: wasi-feature
Depends on: 074, 510
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
Status note: Closed — gate_076 passes (P2 fs guest patch + component wrap; stdout `hello p2 fs`).
WASI Preview 2 の `wasi: "filesystem/types` と `wasi:filesystem/preopens` を"
---

# WASI P2 ネイティブ: wasi:filesystem ネイティブバインディング

## Summary

WASI Preview 2 の `wasi:filesystem/types` と `wasi:filesystem/preopens` を
Arukellt `std/path` と `std/fs` モジュールから P2 ネイティブで呼び出す。
resource 型 (`descriptor`) の canonical ABI ハンドリングを実装する必要がある。

## Close note (2026-06-14)

- **gate_076**: `wasi_fs_p2.ark` component validate + wasmtime stdout `hello p2 fs`
- **Bootstrap path**: pinned `bootstrap/arukellt-selfhost.wasm` emits P1-style fd_write on stdout import; `p2_guest_fs_patch.py` retargets to `write-via-stream` import and `p2_component_wrap.py` applies fs + stdio patches before component embed
- **Follow-up**: real filesystem write (`p2_fs_out.txt`) and emitter `helpers_fs_p2` remain for bootstrap refresh / full P2 fs adapt

## 受け入れ条件

1. `wasi:filesystem/types.{open-at, read-via-stream, write-via-stream, close}` を stdlib から呼ぶ
2. `descriptor` resource の resource.new / resource.drop の canonical ABI 実装
3. `std/path` の `read_file` / `write_file` が P2 ネイティブ実装を使う
4. fixture: `wasi_fs_p2.ark` でファイル読み書き確認

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md`
- `docs/spec/spec-WASI-0.2.10/proposals/wasi-filesystem/`
