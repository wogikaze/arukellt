# Repo Hygiene: generated index の stale 検出を追加する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 424
**Depends on**: 417
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 8

## Summary

docs index、issue index、dependency graph などの generated index が古いまま commit されるのを防ぐ。current-state だけでなく index 類も freshness 対象に含める。

## Current state

- 再生成漏れの検出は docs の一部に偏っている。
- issue index / dependency graph の freshness は別扱いになりやすい。
- generated index が stale でもすぐ気付けない。

## Acceptance

- [ ] index freshness を検出するチェックが追加される。
- [ ] issue index と docs index の両方を対象にする。
- [ ] 差分があれば CI または hook が警告/失敗する。
- [ ] 再生成コマンドが出力される。

## References

- ``issues/open/index.md``
- ``issues/open/dependency-graph.md``
- ``scripts/generate-issue-index.sh``
- ``scripts/check-docs-consistency.py``
