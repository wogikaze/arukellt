# Stdlib: component / WIT helper の実用性を見直す

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 391
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 9

## Summary

component / WIT family を「存在する helper」から「実際に component 周辺コードで再利用したくなる helper」へ引き上げる。world / interface / type 生成や変換支援の surface を見直し、docs examples と合わせて実用面を作る。

## Current state

- component / WIT family はあるが、どこまでを標準ライブラリの責務とするかが読み取りづらい。
- helper が低レベル寄りで、cookbook や examples に繋がりにくい。
- component-model track の現状と stdlib helper surface が十分に結びついていない。

## Acceptance

- [ ] component / WIT helper の責務境界が docs に明記される。
- [ ] 少なくとも 2 つ以上の利用例が追加される。
- [ ] helper ごとに fixture または integration test が用意される。
- [ ] 未成熟な helper は stability tier が見直される。

## References

- ``std/component/**``
- ``std/wit/**``
- ``tests/fixtures/``
- ``docs/stdlib/reference.md``
