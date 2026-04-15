# Stdlib: canonical naming / module layering / surface consistency の第2監査

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 517
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

過去に monomorphic deprecation や naming 整理は進んだが、family 間の命名・返り値規約・module layering はまだ揺れている。
`hashmap_*`, `json_*`, `toml_*`, `env::var/get_var`, `text::concat` vs prelude `concat` などの不一致を再監査し、
canonical naming と alias/deprecation 計画を更新する。

## Repo evidence

- `std/env/mod.ark` に `var` と `get_var` の二重 surface がある
- `std/text` family と prelude の `concat` / formatting helpers が並立している
- collections family に monomorphic historical naming が残る

## Acceptance

- [ ] family ごとの canonical function naming policy が再定義される
- [ ] alias と historical names の整理対象が一覧化される
- [ ] rename / deprecation / keep-as-is の 3 分類が各 API に付与される
- [ ] generated docs と search index に必要な metadata 拡張の要否が判断される

## Primary paths

- `std/env/mod.ark`
- `std/text/`
- `std/prelude.ark`
- `std/collections/`
- `std/manifest.toml`

## References

- `issues/done/359-stdlib-monomorphic-deprecation.md`
- `issues/done/399-stdlib-docs-canonical-name-search-index.md`
