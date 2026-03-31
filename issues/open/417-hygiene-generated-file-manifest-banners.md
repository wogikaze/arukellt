# Repo Hygiene: generated file manifest と ownership banner を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 417
**Depends on**: 373
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 1

## Summary

生成物を人が見分けられるようにし、どのスクリプトがどのファイルを所有するかを manifest と banner で管理する。docs だけでなく issue index などの generated artifact も対象にする。

## Current state

- generated / handwritten の境界は一部文書でしか明示されていない。
- どの generator がどのファイルを作るかを一覧で見られない。
- 手編集禁止が暗黙だと事故が起こりやすい。

## Acceptance

- [ ] generated-file manifest が追加される。
- [ ] 主要 generated ファイルに ownership banner が付く。
- [ ] generator と output の対応が一覧化される。
- [ ] CI または hook で banner/manifest の整合が検証される。

## References

- ``scripts/generate-docs.py``
- ``scripts/generate-issue-index.sh``
- ``docs/README.md``
- ``issues/open/index.md``
