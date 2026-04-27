---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 434
Track: playground
Depends on: 433
Orchestration class: implementation-ready
---
# Playground: shared link の version pinning と再現性を確保する
**Blocks v1 exit**: no
**Priority**: 7

## Summary

share link が将来の compiler / parser 更新で壊れないよう、version pinning と fallback を設計・実装する。古い link を開いたときの挙動もここで決める。

## Current state

- share format が未実装で、version 情報も持てていない。
- compiler/formatter の将来変更により再現性が崩れる可能性が高い。
- docs から共有される URL が長寿命であるためには pinning が必要。

## Acceptance

- [x] share link に version 情報が入る。
- [x] バージョン不一致時の挙動が定義される。
- [x] 古い link の fallback / warning が実装される。
- [x] 再現性テストがある。

## References

- ``docs/index.html``
- ``docs/current-state.md``
- ``docs/adr/**``