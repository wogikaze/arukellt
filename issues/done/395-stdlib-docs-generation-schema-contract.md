# Stdlib Docs: 生成スキーマと metadata 契約を強化する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 395
**Depends on**: 366
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 1

## Summary

stdlib docs の品質を上げる前提として、generator が受け取る metadata と各ページに必ず出す項目を固定する。function page、module page、overview、deprecated 注記、target 注記が ad hoc に増えないよう、生成スキーマを定義する。

## Current state

- `scripts/generate-docs.py` は manifest フィールドから複数ページを生成するが、ページ種別ごとの必須項目が明文化されていない。
- metadata が増えても、どこに表示されるかがコード依存になりやすい。
- docs 差分が大きくなっても schema レベルでは検証されない。

## Acceptance

- [x] stdlib docs 用の生成スキーマが文書化される。
- [x] page kind ごとの必須表示項目が定義される。
- [x] generator に schema validation が追加される。
- [x] schema 違反時に CI で失敗する。

## References

- ``scripts/generate-docs.py``
- ``std/manifest.toml``
- ``docs/stdlib/reference.md``
- ``docs/stdlib/modules/*.md``
