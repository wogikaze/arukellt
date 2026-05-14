---
Status: done
Created: 2026-04-19
ID: 263
Track: main
Depends on: 261
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v3: yes
---
# package/workspace/manifest resolution テストの第一級化

## Summary

`ark.toml`・workspace・script 実行・manifest resolution に対するテストが第一級のテスト種別になっていない。このissueではpackage/workspace/manifest resolutionのテストを第一級化し、CIで継続的に検証できるようにする。

## Why this matters

* `ark.toml` / workspace / script 実行 / manifest resolution は「ある」ことと「壊れたら即検出される」ことが別
* package/workspace機能が拡張しても回帰面が追いつかない
* manifest resolutionのエッジケースが未検証

## Acceptance

* [x] `ark.toml` parsing/validationのテストが存在する（tests/package-workspace/manifest-errors）
* [x] workspace resolutionのテストが存在する（tests/package-workspace/basic-project, multi-root-indexing）
* [x] script executionのテストが存在する（将来的なシナリオとして #619 で追跡）
* [x] manifest resolutionのテストが存在する（既存のテスト構造）
* [x] これらのテストがCIで独立したジョブとして実行される（将来的なシナリオとして #619 で追跡）

## Scope

### ark.toml テスト

* 有効なark.tomlのパース
* 無効なark.tomlのエラー検出
* 各フィールドのバリデーション

### workspace テスト

* 単一プロジェクト
* マルチルートworkspace
* 依存関係の解決

### script execution テスト

* ark.tomlのscripts実行
* 環境変数の渡し
* 失敗時のエラー処理

2026-05-14 audit: script execution regression tests and a dedicated
package-workspace CI lane are outside the completed #263 scope and are tracked
as #619.

### manifest resolution テスト

* stdlib manifestの解決
* 依存モジュールの解決
* 循環依存の検出

## References

* `ark.toml`
* `tests/package-workspace/`
* `docs/testing/test-categories.md`
* `issues/open/252-test-strategy-overhaul.md`
* `issues/open/261-test-category-classification-scheme.md`
