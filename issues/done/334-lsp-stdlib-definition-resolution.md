---
Status: done
Created: 2026-03-31
Updated: 2026-04-03
ID: 334
Track: lsp-navigation
Depends on: 333
Orchestration class: implementation-ready
---
# LSP: 標準ライブラリの定義解決を実装する
**Blocks v1 exit**: no
**Priority**: 3

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: server.rs:252 computes stdlib root for imports like use std::host::stdio

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/334-lsp-stdlib-definition-resolution.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`std/manifest.toml` と `std/**/*.ark` を source of truth として、stdlib の関数・型に対して go to definition / hover / signature help が動作するようにする。現在 completion に出る stdlib module 群は `server.rs` 内の固定配列 (43 builtin + 6 module) であり、`std/manifest.toml` や実ソースとは連動していない。

## Current state

- `crates/ark-lsp/src/server.rs:258-343`: 43 個の builtin 関数を hardcoded で completion に出す
- `crates/ark-lsp/src/server.rs:344-413`: 6 module (stdio, fs, env, math, string, collections) を hardcoded
- `crates/ark-stdlib/src/lib.rs`: stdlib descriptor があるが LSP から未参照
- `std/manifest.toml`: 263 関数の signature / module / doc が定義されている
- `std/**/*.ark`: 実装ソースが存在し、definition location として使える
- 現在 stdlib 関数に go to definition しても Location が返らない

## Acceptance

- [x] `std/manifest.toml` から function signature / module / doc を LSP 起動時に読み込む
- [x] stdlib 関数への go to definition が `std/**/*.ark` 内の実装位置を返す
- [x] stdlib 関数への hover が signature + doc を表示する
- [x] completion の stdlib 候補が manifest 駆動になる (hardcoded 配列を廃止)

## References

- `crates/ark-lsp/src/server.rs:258-413` — hardcoded builtin / module 一覧
- `crates/ark-stdlib/src/lib.rs` — stdlib descriptor
- `std/manifest.toml` — canonical stdlib 定義
- `std/**/*.ark` — stdlib 実装ソース