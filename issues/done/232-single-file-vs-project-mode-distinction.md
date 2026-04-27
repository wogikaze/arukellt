---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 232
Track: main
Depends on: 231
Orchestration class: implementation-ready
Blocks v1 exit: yes
Closed 2026-04-09. docs/ark-toml.md updated with single-file vs project mode section. CLI correctly implements both modes: "arukellt compile/run (single-file) and arukellt build (project). LSP falls back to single-file if no ark.toml found."
---
# 単一ファイルモードと project モードの挙動差を明確化・ドキュメント化する

## Summary

単一ファイルモード（`ark run foo.ark`）と project モード（`ark.toml` 有り）の挙動差が不明確である。
CLI・LSP・Tasks が同じルールで動作していないため、「プロジェクトで使うと壊れる」経験が生まれる。
この issue では両モードの挙動差を仕様として明確にし、全ツールを統一する。

## Acceptance

- [x] 単一ファイルモードと project モードの挙動差が仕様として文書化されている
- [x] CLI・LSP・Tasks の全コマンドが両モードで一貫した挙動を示す
- [x] どのモードで動作しているかが診断ログ・LSP ステータスに表示される
- [x] 単一ファイルを project に昇格させる手順が案内されている

## Scope

### 挙動差の仕様化

- 単一ファイルモードでの import 解決・output 先・target デフォルトの定義
- project モードでの同項目の定義
- 両モード間での動作差の完全なリスト

### ツール統一

- CLI の各コマンドがモード検出ロジックを共有していることの確認
- LSP のルート検出が CLI と同じ結果を返すことの確認
- Tasks provider が CLI と同じモード判断をすることの確認

### ユーザーガイダンス

- 単一ファイルから project への移行手順の docs 追加
- `ark init` コマンドによる `ark.toml` 生成（または既存コマンドの確認）

## References

- `issues/open/231-ark-toml-as-project-model-entry-point.md`
- `issues/open/238-unify-project-root-resolution-cli-lsp-tasks.md`

## Completion Note

Closed 2026-04-09. docs/ark-toml.md updated with single-file vs project mode section. CLI correctly implements both modes: arukellt compile/run (single-file) and arukellt build (project). LSP falls back to single-file if no ark.toml found.