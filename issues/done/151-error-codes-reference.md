# 横断 docs: `docs/compiler/error-codes.md` と診断コード一覧の正規化

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 151
**Depends on**: —
**Track**: cross-cutting
**Blocks v1 exit**: no

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
4. `scripts/check-docs-consistency.py` か同等の check に error code doc の整合確認を追加する

## 参照

- `docs/process/roadmap-cross-cutting.md` §6.4
- `docs/compiler/diagnostics.md`
- `crates/ark-diagnostics/src/codes.rs`
