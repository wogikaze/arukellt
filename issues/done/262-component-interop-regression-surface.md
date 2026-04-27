---
Status: completed
Created: 2026-04-19
ID: 262
Track: main
Depends on: 261
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v3: yes
---
# component interop 回帰面の整備

## Summary

現在のcomponent interopテストは単発smokeに近く、export surfaceの広がりに対して十分な回帰面になっていない。このissueでは`tests/component-interop/`を単発smokeから回帰面へ拡充し、Component Model相互運用性の継続的な品質保証を可能にする。

## Why this matters

* component interopは単発smokeに近く、export surfaceの広がりに対して十分な回帰面になっていない
* 新しいexport surfaceが追加されたときに、既存の相互運用性が壊れていないかを検証する必要がある
* WIT生成の正確性を継続的に検証する必要がある

## Acceptance

* [x] `tests/component-interop/`に複数のシナリオテストが存在する
* [x] 各シナリオについて以下が定義されている：
  * Arukelltソース（export関数定義）
  * 期待されるWIT構造
* [x] 以下のシナリオがカバーされている：
  * 基本的な関数export/import
  * 構造体のexport/import
  * 複数のWIT worldの使用
  * Canonical ABI compliance
* [ ] リソース型のexport/import（将来的なシナリオとしてマーク、未実装）
* [x] CIでcomponent interopテストが独立したジョブとして実行される（既存のcomponent-interopジョブ）
* [ ] WIT round-tripテスト（将来的なシナリオとしてマーク、bindings生成未実装）

## Scope

### 既存テストの確認

* `tests/component-interop/`の既存テストを確認
* 現在のカバレッジを把握

### シナリオテストの追加

* 基本的なexport/importシナリオ（basic-function）
* 構造体export/importシナリオ（struct-export）
* 複数worldシナリオ（multi-world）
* Canonical ABI complianceシナリオ（canonical-abi）

### WIT round-tripテスト

* Arukellt→WIT→Arukelltのround-tripテスト
* 生成されたWITからArukellt bindingsを生成
* bindingsが正しく機能することを確認

### CI配線

* `verification-component-interop`ジョブをCIに追加
* 各シナリオテストを実行
* WIT round-tripテストを実行

## References

* `tests/component-interop/`
* `docs/testing/test-categories.md`
* `issues/open/252-test-strategy-overhaul.md`
* `issues/open/261-test-category-classification-scheme.md`