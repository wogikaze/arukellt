---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 363
Track: stdlib-docs
Depends on: —
Orchestration class: implementation-ready
---
# Stdlib Docs: module family ごとの curated overview page を作る
**Blocks v1 exit**: no
**Priority**: 11

## Summary

source-backed で自動生成されている `docs/stdlib/modules/*.md` の上に、module family ごとの curated overview を追加する。overview には用途概要、推奨 API、target 制約、typical usage pattern、関連 module へのリンクを含める。自動生成 reference は exhaustive index として維持し、overview は学習導線として機能させる。

## Current state

- `docs/stdlib/modules/*.md`: 18 module page が source-backed で生成済み
- 各ページは API 一覧中心で、「この module をいつ使うか」「何が推奨か」の導線が弱い
- `docs/stdlib/README.md` は recommended reads として module pages を列挙するが、family overview ではない
- legacy landing page (`core.md`, `io.md`) と current module page の境界が曖昧

## Acceptance

- [x] 主要 module family (core, collections, text, host, io, json) に curated overview セクションが存在する
- [x] overview に用途概要、推奨 API、target 制約、typical usage pattern が含まれる
- [x] overview と generated reference の責務が文書化される (overview = curated, reference = exhaustive)
- [x] legacy landing page が current overview に統合または archive に移動される

## References

- `docs/stdlib/modules/*.md` — 自動生成 module pages
- `docs/stdlib/README.md` — stdlib docs entry
- `docs/stdlib/reference.md` — exhaustive reference
- `scripts/gen/generate-docs.py` — docs 生成