# Stdlib Docs: manifest metadata と生成 docs の整合チェックを拡張する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 403
**Depends on**: 395
**Track**: stdlib-docs
**Blocks v1 exit**: no
**Priority**: 9

## Summary

metadata enrichment 後の docs drift を防ぐため、since / see-also / deprecated-by / stability / target などの表示が生成 docs に反映されているかを機械的に検証する。

## Current state

- 既存の docs consistency は主に再生成差分を見ており、意味レベルの抜けは取り逃しやすい。
- 新しい metadata を増やすと、表示漏れに気付きにくい。
- reference と module page の両方で一貫して出ているかを検証していない。

## Acceptance

- [x] metadata 表示漏れを検出するチェックが追加される。
- [x] reference と module page の両方を検証する。
- [x] stability / target / deprecated 情報の不整合が fail する。
- [x] 差分が具体的に表示される。

## References

- ``scripts/check-docs-consistency.py``
- ``scripts/generate-docs.py``
- ``std/manifest.toml``
- ``docs/stdlib/reference.md``
