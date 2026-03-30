# ark.toml を project model の唯一の正規入口として仕様化する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 231
**Depends on**: none
**Track**: main
**Blocks v1 exit**: yes

## Summary

`ark.toml` が現状では補助ファイル的な扱いになっており、project model の正規入口として機能していない。
CLI・LSP・タスクランナーが `ark.toml` を一貫して参照していないため、
「プロジェクトを組織できる言語」になれていない。
この issue では `ark.toml` の仕様と、それを中心とした project model を確立する。

## Acceptance

- [ ] `ark.toml` のスキーマ仕様が文書化されている（必須フィールド・オプショナルフィールド・デフォルト値）
- [ ] CLI の全コマンドが `ark.toml` を参照して動作する
- [ ] `ark.toml` がない場合の単一ファイルモードとの違いが明確に定義されている
- [ ] スキーマ違反時に actionable なエラーメッセージが出る

## Scope

### スキーマ仕様設計

- `[package]`・`[workspace]`・`[dependencies]`・`[scripts]`・`[targets]` の各セクションの仕様
- 必須フィールドとオプショナルフィールドの定義
- バージョニング方針（スキーマの互換性保証）

### CLI との統合

- 全 CLI コマンドが `ark.toml` を起点に動作するよう統一
- `ark.toml` 探索アルゴリズム（カレントディレクトリから上方探索）の実装・仕様化

### バリデーション

- `ark.toml` の parse エラー・スキーマ違反の診断実装
- `ark check-manifest` コマンド（または同等機能）の追加

## References

- `docs/language/spec.md`
- `issues/open/232-single-file-vs-project-mode-distinction.md`
- `issues/open/233-module-package-workspace-resolution-spec.md`
