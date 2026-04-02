# Language Docs: spec と guide の同期ポイントを CI で検証する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 370
**Depends on**: 368, 369
**Track**: language-docs
**Blocks v1 exit**: no
**Priority**: 10

## Summary

spec.md と current-first ガイドの間で、stable な機能の記述が食い違わないことを CI で検証する仕組みを作る。spec 変更時にガイドも更新すべき箇所を自動検出し、docs drift を防ぐ。

## Current state

- `scripts/check/check-docs-consistency.py` が docs の整合性を一部検証している
- spec.md と syntax.md / type-system.md の同期は手動
- spec の stable 機能が変更されてもガイドが更新されない可能性がある

## Acceptance

- [x] CI が spec.md の stable 機能記述とガイドの記述を比較チェックする
- [x] 不整合がある場合に CI が warning を出す
- [x] `scripts/check/check-docs-consistency.py` に言語 docs 同期チェックが追加される
- [x] spec の stable 機能変更時にガイド更新の必要性が自動検出される

## References

- `scripts/check/check-docs-consistency.py` — 整合性チェック
- `docs/language/spec.md` — 規範仕様
