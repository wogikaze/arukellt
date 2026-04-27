---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 376
Track: repo-hygiene
Depends on: 373
Orchestration class: implementation-ready
---
# Repo Hygiene: stale state を CI / pre-push で自動検出する
**Blocks v1 exit**: no
**Priority**: 19

## Summary

generated docs、issue index、manifest 整合の stale 検出を CI/hook で実現。

## Acceptance

- [x] generated docs の再生成漏れが CI で検出される — check-docs-consistency.py → generate-docs.py --check
- [x] issue index が stale の場合 CI が警告する — check_issue_index_freshness() added in #424
- [x] `std/manifest.toml` と実装ソースの不整合が CI で検出される — check-stdlib-manifest.sh in verify-harness
- [x] 新規 stale 検出項目が hook に追加される — verify-harness check #16 (generated boundary), issue index freshness

## Implementation

Covered by combination of: #302 (expanded consistency), #424 (issue index freshness), existing stdlib manifest check, and #373 (generated boundary check).