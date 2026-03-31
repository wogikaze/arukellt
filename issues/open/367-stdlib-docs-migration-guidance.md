# Stdlib Docs: deprecated API の移行ガイドを整備する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 367
**Depends on**: 359
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 17

## Summary

deprecated となる monomorphic API やレガシー命名の関数について、具体的な移行コード例を含むガイドを整備する。`docs/stdlib/prelude-migration.md` の歴史記録を拡張し、各 deprecated API に対して「何に置き換えるか」「コード変更例」「いつ削除されるか」を提供する。

## Current state

- `docs/stdlib/prelude-migration.md`: 歴史的経緯の記録として存在
- deprecated API ごとの具体的な移行コード例が不足
- 削除タイムライン (いつ互換 API を消すか) の方針なし
- #359 の monomorphic deprecation が前提

## Acceptance

- [ ] deprecated API ごとに「旧コード → 新コード」の変換例が文書化される
- [ ] 移行ガイドが `docs/stdlib/` に独立文書として存在する
- [ ] 削除タイムラインの方針 (N 版後に削除等) が文書化される
- [ ] `scripts/generate-docs.py` が deprecated API の reference に移行ガイドへのリンクを挿入する

## References

- `docs/stdlib/prelude-migration.md` — 歴史資料
- `std/manifest.toml` — deprecated_by フィールド (#366)
- `docs/stdlib/reference.md` — reference
