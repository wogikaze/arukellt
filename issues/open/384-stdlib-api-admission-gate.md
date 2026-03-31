# Stdlib: API 追加時の admission gate と family coverage チェックを導入する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 384
**Depends on**: 383
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 2

## Summary

stdlib の拡張を「関数を増やす」作業で終わらせないため、API 追加時の必須条件を自動チェックする。fixture、docs、stability、target 対応、example の最低限を揃えない API は merge できない状態にする。

## Current state

- 新規 API 追加時に fixture / docs / metadata の同時更新を強制する仕組みがない。
- module family ごとのカバレッジ差が大きいが、どこが薄いかを CI が示さない。
- manifest と実装がそろっていても、example や recipe が無いまま公開される余地がある。

## Acceptance

- [ ] 新規 stdlib API に対する admission checklist が機械可読な形で定義される。
- [ ] CI が fixture / docs / metadata の欠落を検出する。
- [ ] family ごとの API 数・fixture 数・docs 数を出す coverage report が生成される。
- [ ] admission gate を通らない API 追加が fail する。

## References

- ``std/manifest.toml``
- ``tests/fixtures/``
- ``docs/stdlib/reference.md``
- ``scripts/check-docs-consistency.py``
- ``docs/stdlib/stability-policy.md``
