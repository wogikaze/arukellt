---
Status: completed
Created: 2026-04-19
ID: 265
Track: main
Depends on: 261
Orchestration class: design-ready
Orchestration upstream: —
---

# 失敗層の即時特定
**Blocks v3**: yes

## Summary

テスト失敗時にlanguage/backend/tooling regressionを区別できる命名・レポートの仕組みを確立する。

## Why this matters

* テスト命名とカテゴリが整理されておらず、どの失敗がlanguage/backend/tooling regressionかを一目で追えない
* 失敗時に「どの層が壊れたか」が直ちに分かる必要がある

## Acceptance

* [x] テストカテゴリが定義されている（261完了）
* [x] 各カテゴリの責務・対象・合否基準が定義されている（261完了）
* [x] CI上で各カテゴリが独立したジョブとして構成されている（264完了）
* [ ] テスト失敗時にカテゴリが明示される（将来的なシナリオ）
* [ ] 失敗レポートにカテゴリ情報が含まれる（将来的なシナリオ）

## Scope

### テスト命名規則

* 各カテゴリのテスト命名規則を定義（261で完了）

### CIレポート

* 失敗時にカテゴリを明示する（将来的なシナリオ）

## References

* `docs/testing/test-categories.md`
* `issues/open/252-test-strategy-overhaul.md`
* `issues/open/261-test-category-classification-scheme.md`
