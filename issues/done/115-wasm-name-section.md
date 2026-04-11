# Wasm Name Section: デバッグ用関数名・ローカル名セクション生成

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-04
**ID**: 115
**Depends on**: —
**Track**: wasm-quality
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/115-wasm-name-section.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

生成する Wasm バイナリに Name Section (custom section `name`) を追加し、
wasmtime のスタックトレースや `wasm-objdump` でのデバッグ体験を改善する。
`--opt-level 0` では名前情報を完全に含め、`--opt-level 2` では省略可能とする。

## 受け入れ条件

1. T3 emitter が Name Section に Ark 関数名をエクスポート
2. ローカル変数にも名前を付与 (`--opt-level 0` でのみ)
3. wasmtime のスタックトレースで Ark 関数名が表示されることを確認
4. `--strip-debug` フラグで Name Section を省略

## 参照

- WebAssembly binary format §custom section
