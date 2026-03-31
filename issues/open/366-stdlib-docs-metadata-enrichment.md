# Stdlib Docs: manifest metadata を enrichment し生成 docs の情報密度を上げる

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 366
**Depends on**: —
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 14

## Summary

`std/manifest.toml` の各関数エントリに、生成 docs が自動的に豊かになるメタデータを追加する。具体的には、since version、see-also (関連関数)、category 分類、example snippet、deprecated-by (後継関数) などのフィールドを manifest に持たせ、`scripts/generate-docs.py` が活用する。

## Current state

- `std/manifest.toml`: name, module, params, return_type, doc, stability, kind, target が主要フィールド
- since version (いつ追加されたか)、see-also、deprecated-by フィールドなし
- `scripts/generate-docs.py` は現在のフィールドのみを使って docs を生成
- category は `doc_category` フィールドで一部対応済みだが全関数には付与されていない

## Acceptance

- [ ] manifest に `since`、`see_also`、`deprecated_by` フィールドが定義される
- [ ] 全関数に `doc_category` が付与される
- [ ] `scripts/generate-docs.py` が新フィールドを reference / module pages に反映する
- [ ] 生成 docs で「いつ追加されたか」「関連関数」「後継関数」が表示される

## References

- `std/manifest.toml` — manifest 定義
- `scripts/generate-docs.py` — docs 生成
- `docs/stdlib/reference.md` — 生成先
