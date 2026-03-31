# Playground: shared link の version pinning と再現性を確保する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 434
**Depends on**: 433
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 7

## Summary

share link が将来の compiler / parser 更新で壊れないよう、version pinning と fallback を設計・実装する。古い link を開いたときの挙動もここで決める。

## Current state

- share format が未実装で、version 情報も持てていない。
- compiler/formatter の将来変更により再現性が崩れる可能性が高い。
- docs から共有される URL が長寿命であるためには pinning が必要。

## Acceptance

- [ ] share link に version 情報が入る。
- [ ] バージョン不一致時の挙動が定義される。
- [ ] 古い link の fallback / warning が実装される。
- [ ] 再現性テストがある。

## References

- ``docs/index.html``
- ``docs/current-state.md``
- ``docs/adr/**``
