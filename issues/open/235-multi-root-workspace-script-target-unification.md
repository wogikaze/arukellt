# コンパイラ/CLI/LSP ツール層: multi-root workspace・script 実行・target 設定の解決統一

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 235
**Depends on**: 232, 233
**Track**: main
**Blocks v1 exit**: no

## Summary

multi-root workspace・script 実行・target 設定において、コンパイラ/CLI/LSP ツール層が同じ解決結果を返すことを保証する。
VS Code 拡張機能 UI 側の設定は #215 で扱う。この issue は**コンパイラ・CLI・LSP サーバー実装層**に限定する。

CLI と LSP が同じ workspace root を発見し、同じ target を参照し、同じ script 一覧を返すことで、
「CLI では通るが LSP ではエラー」「Tasks が別の target を見ている」問題を解消する。

## Acceptance

- [ ] multi-root workspace で CLI と LSP サーバーが同じルートを発見する
- [ ] `ark.toml` の `[scripts]` セクションのスクリプト一覧が CLI と LSP で一致する
- [ ] `[targets]` の設定が CLI コンパイルと LSP の型チェックで一致している
- [ ] workspace member ごとの設定分離が正しく動作する

## Scope

### multi-root workspace（ツール層）

- CLI と LSP サーバーの workspace root 発見ロジックを同一実装に統一
- workspace member のそれぞれの `ark.toml` を独立して扱う実装
- root 発見の結果が CLI・LSP で再現可能であることのテスト

### script 解決の統一（ツール層）

- `ark.toml` の `[scripts]` を CLI と LSP サーバーが同じパーサーで読む
- script 定義の変更が LSP 再読み込みで反映される仕組み

### target 設定の統一（ツール層）

- `[targets]` セクションの優先順位（CLI フラグ > ark.toml > デフォルト）の仕様化と実装
- LSP サーバーが target 設定を参照して適切な型チェックを行うことの確認

## References

- `issues/open/231-ark-toml-as-project-model-entry-point.md`
- `issues/open/232-single-file-vs-project-mode-distinction.md`
- `issues/open/215-multi-root-workspace-and-per-project-config.md` （拡張機能 UI は 215）
- `issues/open/238-unify-project-root-resolution-cli-lsp-tasks.md`
