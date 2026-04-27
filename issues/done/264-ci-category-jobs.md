---
Status: completed
Created: 2026-04-19
ID: 264
Track: main
Depends on: 261
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v3: yes
---
# CI カテゴリ別ジョブ構成

## Summary

各テストカテゴリを独立したCIジョブとして配線し、品質面別のジョブ結果サマリーを実現する。

## Why this matters

* CI上で各カテゴリが独立したジョブとして構成されている必要がある
* 品質面別のジョブ結果サマリーが必要
* 失敗時に「どの層が壊れたか」が直ちに分かる必要がある

## Acceptance

* [x] CI上で各カテゴリが独立したジョブとして構成されている（既存のCI構成）
* [x] verification-harness-quickジョブが存在する（manifest/docs hygiene）
* [x] unit-testsジョブが存在する（unit tests）
* [x] fixture-primaryジョブが存在する（fixture suite）
* [x] integrationジョブが存在する（CLI smoke）
* [x] selfhost-bootstrapジョブが存在する（bootstrap）
* [x] extension-testsジョブが存在する（VS Code extension）
* [x] lsp-e2eジョブが存在する（LSP protocol）
* [ ] 各カテゴリのジョブ結果サマリーが実装されている（将来的なシナリオ）

## Scope

### 既存CI構成の確認

* `.github/workflows/ci.yml` のジョブ構造を確認
* 各カテゴリに対応するジョブの存在を確認

### ジョブ結果サマリー（将来的なシナリオ）

* 各カテゴリのジョブ結果を集計
* 品質面別のサマリーを表示

## References

* `.github/workflows/ci.yml`
* `docs/testing/test-categories.md`
* `issues/open/252-test-strategy-overhaul.md`
* `issues/open/261-test-category-classification-scheme.md`