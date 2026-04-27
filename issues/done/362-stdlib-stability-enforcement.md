---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 362
Track: stdlib-api
Depends on: 358, 360
Orchestration class: implementation-ready
Blocks v1 exit: False
Priority: 9
# Stdlib: stability tier を実行時・CI で検証する
---
# Stdlib: stability tier を実行時・CI で検証する

## Summary

`stable / provisional / experimental / host_stub` の stability tier を、manifest metadata としてだけでなく、CI / compiler / docs 生成で自動検証する仕組みに引き上げる。experimental API を stable と同列に公開しない、host_stub を fixture なしで stable 表記しない、といった制約を自動で守らせる。

## Acceptance

- [x] CI が `host_stub` の関数に対応する fixture がないことを検出し警告する
- [x] `experimental` API の使用時に compiler が W-level diagnostic を出す (opt-in)
- [x] `docs/stdlib/reference.md` の生成時に stability tier ごとにセクションが分離される
- [x] stability tier の変更 (experimental → stable) に対応する checklist が文書化される
- [x] `scripts/check/check-docs-consistency.py` が stability metadata と実装状態の不整合を検出する

## References

- `std/manifest.toml` — stability フィールド
- `docs/stdlib/stability-policy.md` — tier 定義
- `scripts/check/check-docs-consistency.py` — 整合性チェック
- `scripts/gen/generate-docs.py` — docs 生成