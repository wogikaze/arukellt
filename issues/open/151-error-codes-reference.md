# 横断 docs: `docs/compiler/error-codes.md` と診断コード一覧の正規化

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-03
**ID**: 151
**Depends on**: —
**Track**: cross-cutting
**Blocks v1 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/151-error-codes-reference.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`docs/process/roadmap-cross-cutting.md` §6.4 は `docs/compiler/error-codes.md` を要求している。
現状の `docs/compiler/diagnostics.md` はカテゴリ説明として有用だが、
`crates/ark-diagnostics` の registry と 1 対 1 に対応する正規の error code reference ではない。

## 受け入れ条件

1. `docs/compiler/error-codes.md` が追加され、主要 diagnostic code の code / severity / phase / message を一覧できる
2. `crates/ark-diagnostics/src/codes.rs` の registry と文書の対応関係が明確になる
3. `docs/compiler/diagnostics.md` から `error-codes.md` にリンクされる
4. docs consistency check で code doc の欠落や主要ズレを検出できる

## 実装タスク

1. `crates/ark-diagnostics` の code registry を棚卸しする
2. 手書きか生成かを決め、更新フローを明文化する
3. `docs/compiler/error-codes.md` を追加し、代表 code と運用ルールを整理する
4. `scripts/check/check-docs-consistency.py` か同等の check に error code doc の整合確認を追加する

## 参照

- `docs/process/roadmap-cross-cutting.md` §6.4
- `docs/compiler/diagnostics.md`
- `crates/ark-diagnostics/src/codes.rs`
