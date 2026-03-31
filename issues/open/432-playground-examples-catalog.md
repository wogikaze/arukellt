# Playground: examples catalog を docs / fixtures と共有する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 432
**Depends on**: 381
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 5

## Summary

playground の examples を独自手入力で持たず、docs examples や fixtures と共有する catalog を作る。サンプルごとにカテゴリ、target 制約、host capability、難易度を持たせる。

## Current state

- examples loader の元データがまだない。
- fixtures / docs examples / cookbook と playground samples が二重管理になる危険がある。
- host capability を使う例と pure example を分けたい。

## Acceptance

- [ ] examples catalog が追加される。
- [ ] fixtures / docs examples と共有できる構造になる。
- [ ] カテゴリと capability 情報が入る。
- [ ] playground が catalog からサンプルを読み込める。

## References

- ``tests/fixtures/**``
- ``docs/examples/**``
- ``docs/stdlib/cookbook.md``
