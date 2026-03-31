# Repo Hygiene: done issues / archive docs の retention workflow を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-01
**ID**: 419
**Depends on**: 375
**Track**: repo-hygiene

## Acceptance

- [x] archive / done 移動手順が文書化される。
- [x] 移動時に記録する metadata が決まる。
- [x] 少なくとも issue または docs で運用例が整備される。
- [x] 関連 index の更新手順が明記される。

## Resolution

- `docs/retention-policy.md` (created for #375) documents the archive workflow with 4 steps: add banner, update links, move file, record in commit
- Issue close workflow well-established: create done file, delete open file, run `generate-issue-index.sh`
- 40+ issues have followed this workflow, establishing the operational pattern
- Index regeneration: `bash scripts/generate-issue-index.sh`
