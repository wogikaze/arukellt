# Stdlib: module family ごとの maturity scoreboard を生成する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 393
**Depends on**: 383
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 11

## Summary

family ごとの拡張優先順位を議論しやすくするため、API 数・fixture 数・recipe 数・stability 分布・host dependency をまとめた scoreboard を自動生成する。方針 issue だけではなく、毎回の判断材料を機械的に出すことが目的。

## Current state

- family ごとの成熟度は感覚的にしか見えず、拡張対象の優先順位が会話依存になりやすい。
- `std/manifest.toml` と docs / tests の情報が別れており、横断集計がない。
- policy は書けても、現状との差分が見えにくい。

## Acceptance

- [ ] family scoreboard が自動生成される。
- [ ] API 数・fixture 数・recipe 数・stability 分布が出る。
- [ ] docs または current-state から参照できる。
- [ ] 不足 coverage を示す項目がある。

## References

- ``std/manifest.toml``
- ``tests/fixtures/``
- ``docs/stdlib/README.md``
- ``scripts/generate-docs.py``
