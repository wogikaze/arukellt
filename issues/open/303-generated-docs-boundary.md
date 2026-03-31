# 生成 docs と手書き docs の境界を明文化する

**Status**: open
**Created**: 2026-03-31
**ID**: 303
**Depends on**: —
**Track**: main
**Priority**: 23

## Summary

`generate-docs.py` が 30+ ファイルを生成するが、どれが生成でどれが手書きか分かりにくい。手書き docs を誤って再生成で上書きするリスクがある。

## Current state

- `scripts/generate-docs.py`: 1103 行、stdlib module pages / sidebar / landing pages を生成
- `docs/current-state.md`: 5 箇所がインラインマーカーで自動更新、残りは手書き
- 生成ファイル一覧が明文化されていない

## Acceptance

- [ ] `scripts/generate-docs.py` の冒頭コメントに生成ファイル一覧がある
- [ ] 各生成ファイルの先頭に `<!-- This file is auto-generated. Do not edit manually. -->` バナーがある
- [ ] docs の README に「生成 vs 手書き」の分類表がある

## References

- `scripts/generate-docs.py`
- `docs/README.md`
