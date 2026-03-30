# Multi-root workspace + per-project configuration

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 215
**Depends on**: 202
**Track**: parallel
**Blocks v1 exit**: no

## Summary

multi-root workspace 対応、workspace folder ごとの target / emit / adapter 設定、project 単位 compile / run / test 導線、manifest 不在時と有り時の挙動分離、unknown script エラー整備を実装する。

現状は単一 workspace folder を想定した実装で、multi-root workspace や複数 `ark.toml` を持つプロジェクトへの対応が不十分。

## Acceptance

- [ ] multi-root workspace で各 folder を独立した project として認識できる
- [ ] workspace folder ごとに target / emit / adapter を設定できる
- [ ] manifest（ark.toml）不在時と有り時で挙動が適切に分離されている

## Scope

### Multi-root workspace
- workspace folder ごとの project root 自動検出
- `ark.toml` の有無による project 識別
- folder 追加 / 削除時の動的再認識

### Per-project configuration
- `settings.json` での folder-scoped 設定（target / emit / adapter / binary path）
- workspace defaults vs. folder overrides の優先順位
- `.vscode/settings.json` による per-project オーバーライド

### Compile / run / test 導線
- アクティブファイルの所属 project を自動判定
- project 単位での compile / run / test task 生成
- project 選択 quick pick（multi-root 時）

### Manifest 分岐
- `ark.toml` 不在: 単一ファイルモード、minimum 設定
- `ark.toml` あり: package / workspace / tool defaults 読み取り
- `ark.toml` 形式エラー時の診断表示

## References

- `issues/open/202-ark-toml-schema-and-project-workspace-discovery.md`
- `issues/open/188-ark-toml-project-workspace-and-scripts.md`
- `issues/open/203-script-run-and-script-list-cli-surface.md`
- `extensions/arukellt-all-in-one/src/`
