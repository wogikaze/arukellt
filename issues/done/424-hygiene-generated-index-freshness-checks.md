# Repo Hygiene: generated index の stale 検出を追加する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 424
**Depends on**: 417
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 8

## Summary

docs index、issue index の freshness を自動検出。

## Acceptance

- [x] index freshness を検出するチェックが追加される
- [x] issue index と docs index の両方を対象にする
- [x] 差分があれば CI / hook が警告する
- [x] 再生成コマンドが出力される

## Implementation

- Extended `scripts/check/check-docs-consistency.py` with `check_issue_index_freshness()`
- Regenerates issue indexes, compares output, reports stale files with command to fix
