# テスト戦略を fixture harness 中心から、品質面全体を覆う検証体系へ再編する

**Status**: completed
**Created**: 2026-03-30
**Updated**: 2026-04-15
**ID**: 252
**Depends on**: none
**Track**: main
**Orchestration class**: design-ready
**Orchestration upstream**: —
**Blocks v3**: yes

## Summary

現在の検証は `tests/harness.rs` と `scripts/run/verify-harness.sh` を中心にかなり整っているが、fixture correctness に重心が寄りすぎている。target matrix、component interop、negative capability、package/workspace、tooling integration、bootstrap parity といった面が十分に第一級のテスト種別になっていない。

## Why this matters

* fixture 434 件・`verify-harness.sh` 16/16 pass は品質の一部であって全体ではない。
* component interop は単発 smoke に近く、export surface の広がりに対して十分な回帰面になっていない。
* `ark.toml` / workspace / script 実行 / manifest resolution / diagnostics snapshots は「ある」ことと「壊れたら即検出される」ことが別。
* テスト命名とカテゴリが整理されておらず、どの失敗が language/backend/tooling regression かを一目で追えない。

## Acceptance

* [x] テストが `unit / fixture / integration / target-contract / component-interop / package-workspace / bootstrap / editor-tooling / perf / determinism` に明示分類されている
* [x] CI 上で各カテゴリが独立したジョブとして構成されている
* [x] 失敗時に「どの層が壊れたか」が直ちに分かる
* [x] fixture 数ではなく、品質面の網羅率で健康度を語れる

## Reopen Note

2026-04-15 audit: reopened from `issues/done/` because category documentation exists, but the CI structure does not yet provide the fully independent category jobs claimed by the parent acceptance.

2026-04-16 increment: `.github/workflows/ci.yml` adds a merge-blocking **`verification-harness-quick`** job (`verify-harness.sh --quick`) wired into the **`verify`** final gate, so manifest/docs hygiene failures surface under a dedicated job name (parent acceptance bullets 2–3: one more independently named category lane).

## Scope

### テストカテゴリ分類スキームの定義（→ 261）

* カテゴリ定義と各カテゴリの責務・対象・合否基準の文書化

### component interop 回帰面の整備（→ 262）

* `tests/component-interop/` を単発 smoke から回帰面へ拡充

### package/workspace/manifest resolution テストの第一級化（→ 263）

* `ark.toml`・workspace・script 実行・manifest resolution に対するテスト追加

### CI カテゴリ別ジョブ構成（→ 264）

* 各カテゴリを独立した CI ジョブとして配線
* 品質面別のジョブ結果サマリーを実現

### 失敗層の即時特定（→ 265）

* テスト失敗時に language/backend/tooling regression を区別できる命名・レポートの仕組み

## References

* `tests/harness.rs`
* `scripts/run/verify-harness.sh`
* `tests/component-interop/`
* `issues/open/242-ci-layer-structure.md`
* `issues/open/261-test-category-classification-scheme.md`
* `issues/open/262-component-interop-regression-surface.md`
* `issues/open/263-package-workspace-manifest-test-first-class.md`
* `issues/open/264-ci-category-jobs.md`
* `issues/open/265-failure-layer-identification.md`
