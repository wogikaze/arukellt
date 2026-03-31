# Repo Hygiene: orphan / stale file inventory を作るスクリプトを追加する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 418
**Depends on**: —
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 2

## Summary

repo 内の孤立ファイル、参照されない historical asset、古い中間生成物候補を機械的に洗い出す inventory スクリプトを作る。cleanup を感覚ではなく定期レポートで進めるための基盤。

## Current state

- 不要ファイルの議論は手動 inspection に寄りやすい。
- archive すべきか削除すべきか判断する材料が自動で出ない。
- zip / baseline / sample などの大型ファイルも蓄積しやすい。

## Acceptance

- [ ] orphan/stale inventory スクリプトが追加される。
- [ ] 少なくとも docs / tests / benchmarks / artifacts を走査する。
- [ ] レポートに候補ファイルと参照状況が出る。
- [ ] CI か定期手順から呼び出せる。

## References

- ``docs/**``
- ``tests/**``
- ``benchmarks/**``
- ``scripts/**``
