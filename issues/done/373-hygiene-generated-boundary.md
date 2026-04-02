# Repo Hygiene: generated と hand-written の境界を明確化し ownership を付与する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 373
**Depends on**: —
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 4

## Summary

generated ファイルと手書きファイルの境界を明確にし、ownership banner を付与。

## Acceptance

- [x] 全 generated ファイルに ownership banner が付与される
- [x] generated ファイルの一覧が `.generated-files` manifest に文書化される
- [x] CI が generated ファイルの手編集を検出し警告する
- [x] `scripts/gen/generate-docs.py` が banner を自動挿入する

## Implementation

- `.generated-files`: 39 entries with path | generator | command format
- `scripts/check/check-generated-files.sh`: validates existence, banners, unlisted files
- Added banner to `docs/_sidebar.md`
- Updated `scripts/gen/generate-docs.py` to include sidebar banner
- Integrated as verify-harness check #16
