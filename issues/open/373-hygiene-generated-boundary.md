# Repo Hygiene: generated と hand-written の境界を明確化し ownership を付与する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 373
**Depends on**: —
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 4

## Summary

docs / index / reference で自動生成されているファイルと、手書きで管理しているファイルの境界を明確にし、generated ファイルに ownership banner (生成元スクリプト、再生成コマンド、手編集禁止の注意) を付与する。手編集した generated ファイルが次回の生成で上書きされる事故を防ぐ。

## Current state

- `scripts/generate-docs.py` が多数の docs を生成しているが、対象一覧がコードを読まないと分からない
- 一部の generated ファイルに `<!-- generated -->` バナーがあるが、全ファイルには付いていない
- `issues/open/index.md` / `dependency-graph.md` は `generate-issue-index.sh` で生成
- generated と手書きの境界を CI で検証する仕組みが部分的 (`check-docs-consistency.py`)

## Acceptance

- [ ] 全 generated ファイルに ownership banner (生成元、再生成コマンド、手編集禁止) が付与される
- [ ] generated ファイルの一覧が文書化される (or `.generated-files` manifest)
- [ ] CI が generated ファイルの手編集を検出し警告する
- [ ] `scripts/generate-docs.py` が banner を自動挿入する

## References

- `scripts/generate-docs.py` — docs 生成
- `scripts/generate-issue-index.sh` — issue index 生成
- `scripts/check-docs-consistency.py` — 整合性チェック
