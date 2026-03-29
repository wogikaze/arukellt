# VS Code: `arukellt-all-in-one` 拡張の基盤整備

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 184
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

`arukellt-all-in-one` の最初の責務は、既に存在する `arukellt lsp` / `ark-lsp` を VS Code から迷わず使えるようにすることと、Arukellt プロジェクトの基本操作を 1 つの拡張に集約することである。

現在の CLI には `lsp` subcommand があり、LSP 側も diagnostics / hover / completion / definition / references / documentSymbol / semanticTokens を提供している。一方で、VS Code 拡張そのもの、CLI binary discovery、出力パネル、task / command wiring、feature detection、導入ガイドは未整備である。

## 受け入れ条件

1. VS Code 拡張が `.ark` を Arukellt language として登録し、comment rules / brackets / auto-closing / snippets / basic grammar を提供する
2. 拡張が `arukellt lsp` を起動し、diagnostics / hover / completion / definition / references / document symbols / semantic tokens を利用できる
3. `arukellt` binary path を自動検出でき、失敗時は settings で上書きできる
4. Command Palette / status bar / output channel から `check` / `compile` / `run` / `restart language server` を実行できる
5. target / emit / adapter などの代表設定を VS Code settings から渡せる
6. 外部依存や未対応機能がある場合、無言失敗ではなく actionable message を出す

## 実装タスク

1. `arukellt-all-in-one` extension package を作成する
2. language configuration / snippets / grammar / icon / file association を定義する
3. language client を実装し、`arukellt lsp` stdio transport と接続する
4. CLI path discovery と version check を実装する
5. compile / run / check 系 command と task provider を実装する
6. README / setup guide / troubleshooting を追加する

## 参照

- `crates/ark-lsp/src/lib.rs`
- `crates/arukellt/src/main.rs`
- `crates/arukellt/src/commands.rs`
- `docs/current-state.md`
- `docs/contributing.md`
