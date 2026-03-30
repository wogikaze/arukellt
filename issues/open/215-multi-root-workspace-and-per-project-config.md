# VS Code extension: multi-root workspace + per-project configuration UI

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 215
**Depends on**: 202
**Track**: parallel
**Blocks v1 exit**: no

## Summary

VS Code 拡張機能の multi-root workspace 対応と、workspace folder ごとの設定 UI を実装する。
manifest 不在時と有り時の挙動分離、unknown script エラー整備を含む。

コンパイラ/CLI/LSP ツール層の project root 解決統一は #238 で扱う。
この issue は**拡張機能 UI 側**（VS Code の workspaceFolder API・設定・task 生成）に限定する。

## Acceptance

- [ ] multi-root workspace で各 folder を独立した project として認識できる
- [ ] workspace folder ごとに target / emit / adapter を VS Code 設定から上書きできる
- [ ] manifest（ark.toml）不在時と有り時で拡張機能の挙動が適切に分離されている

## Scope

### Multi-root workspace（拡張機能 UI 層）

- workspace folder ごとの project root 自動検出（VS Code workspaceFolder API）
- `ark.toml` の有無による project 識別
- folder 追加 / 削除時の動的再認識

### Per-project configuration（VS Code 設定）

- `settings.json` での folder-scoped 設定（target / emit / adapter / binary path）
- workspace defaults vs. folder overrides の優先順位
- `.vscode/settings.json` による per-project オーバーライド

### Compile / run / test 導線

- アクティブファイルの所属 project を自動判定
- project 単位での compile / run / test task 生成
- project 選択 quick pick（multi-root 時）

### Manifest 分岐（拡張機能の表示・診断）

- `ark.toml` 不在: 単一ファイルモード、minimum 設定
- `ark.toml` あり: package / workspace / tool defaults 読み取り
- `ark.toml` 形式エラー時の診断表示

## References

- `issues/open/202-ark-toml-schema-and-project-workspace-discovery.md`
- `issues/open/238-unify-project-root-resolution-cli-lsp-tasks.md` （ツール層の統一は 238）
- `issues/open/188-ark-toml-project-workspace-and-scripts.md`
- `extensions/arukellt-all-in-one/src/`
