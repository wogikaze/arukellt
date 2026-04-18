# テストカテゴリ分類スキームの定義

**Status**: completed
**Created**: 2026-04-19
**ID**: 261
**Depends on**: none
**Track**: main
**Orchestration class**: design-ready
**Orchestration upstream**: —
**Blocks v3**: yes

## Summary

現在のテストはfixture correctnessに重心が寄りすぎており、target matrix、component interop、negative capability、package/workspace、tooling integration、bootstrap parityといった面が第一級のテスト種別になっていない。このissueではテストを明示的に分類するスキームを定義し、各カテゴリの責務・対象・合否基準を文書化する。

## Why this matters

* テスト命名とカテゴリが整理されておらず、どの失敗がlanguage/backend/tooling regressionかを一目で追えない
* fixture数ではなく、品質面の網羅率で健康度を語れるようにする必要がある
* CI上で各カテゴリを独立したジョブとして構成する前提となる

## Acceptance

* [x] テストカテゴリ定義文書が `docs/testing/test-categories.md` に存在する
* [x] 各カテゴリについて以下が定義されている：
  - カテゴリ名と責務
  - 対象範囲（何をテストするか）
  - 合否基準（どうなればpassするか）
  - テスト命名規則（ファイル名・関数名）
* [x] 以下のカテゴリが定義されている：
  - `unit` — 単体テスト（関数・モジュール単位）
  - `fixture` — フィクスチャテスト（言語機能の正しさ）
  - `integration` — 統合テスト（複数モジュールの連携）
  - `target-contract` — ターゲット契約テスト（T1/T2/T3/T4/T5のABI）
  - `component-interop` — Component Model相互運用性テスト
  - `package-workspace` — パッケージ/ワークスペース/マニフェスト解決テスト
  - `bootstrap` — セルフホストbootstrapテスト
  - `editor-tooling` — エディタツーリング（LSP・DAP）テスト
  - `perf` — 性能テスト（コンパイル時間・実行時間・メモリ・バイナリサイズ）
  - `determinism` — 決定性テスト（同一入力→同一出力）

## Scope

### テストカテゴリ定義文書の作成

* `docs/testing/test-categories.md` を新規作成
* 各カテゴリについて以下を記述：
  - 責務: そのカテゴリが保証する品質面
  - 対象: テスト対象のコード・機能
  - 合否基準: テストの成功/失敗判定基準
  - 命名規則: テストファイル・テスト関数の命名規則
  - CI配線: どのCIジョブで実行されるか

### 既存テストの分類

* `tests/` 以下の既存テストをカテゴリに分類
* 各テストファイルにカテゴリタグを付与（コメントまたはディレクトリ構造）

## References

* `tests/harness.rs`
* `scripts/run/verify-harness.sh`
* `issues/open/252-test-strategy-overhaul.md`
* `issues/open/242-ci-layer-structure.md`
