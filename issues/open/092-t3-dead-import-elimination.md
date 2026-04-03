# T3: 未使用 WASI import の除去

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 092
**Depends on**: —
**Track**: backend-opt
**Blocks v4 exit**: yes


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/092-t3-dead-import-elimination.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

現在の T3 emitter は `fd_write`・`path_open`・`fd_read`・`fd_close` の
4つの WASI 関数を常に import するが、実際に使用しない関数も import される。
例えば、`hello_world.ark` は `path_open`・`fd_read`・`fd_close` を使わない。
未使用 import を除去することでバイナリサイズを削減する。

## 受け入れ条件

1. T3 emitter が使用する WASI 関数のセットをビルド時に追跡
2. ImportSection に実際に使用する関数のみを追加
3. `hello.wasm` から未使用 WASI import が除去されることを `wasm-objdump` で確認
4. `hello.wasm` バイナリサイズが 1KB 以下 (roadmap v4 目標) の達成に貢献

## 参照

- roadmap-v4.md §2 (hello.wasm 1KB 目標)
