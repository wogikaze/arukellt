---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 417
Track: repo-hygiene
Depends on: 373
Orchestration class: implementation-ready
---
# Repo Hygiene: generated file manifest と ownership banner を実装する
**Blocks v1 exit**: no
**Priority**: 1

## Summary

生成物の ownership 管理。#373 で実装済み。

## Acceptance

- [x] generated-file manifest が追加される — `.generated-files`
- [x] 主要 generated ファイルに ownership banner が付く
- [x] generator と output の対応が一覧化される
- [x] CI または hook で banner/manifest の整合が検証される — check #16

## Implementation

Completed as part of #373. See `.generated-files` manifest and `scripts/check/check-generated-files.sh`.