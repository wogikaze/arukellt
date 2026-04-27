---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 303
Track: docs/ops
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 23
---

- `scripts/gen/generate-docs.py`: 1103 行、stdlib module pages / sidebar / landing pages を生成
- `docs/current-state.md`: 5 箇所がインラインマーカーで自動更新、残りは手書き
# 生成 docs と手書き docs の境界を明文化する

## Summary

`generate-docs.py` が 30+ ファイルを生成するが、どれが生成でどれが手書きか分かりにくい。手書き docs を誤って再生成で上書きするリスクがある。

## Current state

- `scripts/gen/generate-docs.py`: 1103 行、stdlib module pages / sidebar / landing pages を生成
- `docs/current-state.md`: 5 箇所がインラインマーカーで自動更新、残りは手書き
- 生成ファイル一覧が明文化されていない

## Acceptance

- [x] `scripts/gen/generate-docs.py` の冒頭コメントに生成ファイル一覧がある
- [x] 各生成ファイルの先頭に `<!-- This file is auto-generated. Do not edit manually. -->` バナーがある
- [x] docs の README に「生成 vs 手書き」の分類表がある

## References

- `scripts/gen/generate-docs.py`
- `docs/README.md`