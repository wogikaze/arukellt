# Playground: permalink / share format と圧縮方式を定義する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 433
**Depends on**: 428
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 6

## Summary

共有 URL を安定にするため、コード、version、flags、selected example などをどうエンコードするか決める。圧縮や長さ制限も含めて format を定め、将来互換を壊しにくくする。

## Current state

- share link は未実装。
- 単純な URL hash だけでは将来の version pinning や flags を扱いにくい。
- 圧縮・長さ制約・互換性の方針が必要。

## Acceptance

- [x] share format の仕様が決まる。
- [x] 少なくともコードと version を含む URL が生成できる。
- [x] 長い入力に対する圧縮または fallback 方針がある。
- [x] round-trip テストがある。

## References

- ``docs/index.html``
- ``docs/adr/**``
