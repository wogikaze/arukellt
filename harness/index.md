# Harness Docs Index

このディレクトリは、近傍の 3 つの言語ツールチェーンプロジェクトから、再利用可能なハーネス要素を抽出してまとめたものです。

- `NEPLg2`
- `vibe-lang`
- `wado`

目的は、各リポジトリの全体像を保存することではありません。agent-driven development、検証、lint、生成ドキュメント、CI を次のプロジェクトに移植できる形で再構成することです。

## 読む順番

1. `project-comparison.md`
   まず各プロジェクトが何を source of truth にしているかを把握する。
2. `blueprint.md`
   次プロジェクト向けの標準ハーネスを決める。
3. `tooling-catalog.md`
   採用するツール、skills、lint、doc 生成、CI パターンを具体化する。

## このハーネス文書群が定義するもの

推奨ハーネスは、次の 8 要素をコアに置きます。

1. 実行可能な source of truth へ案内する短い入口文書
2. 進行中作業を明示する queue または issue index
3. ワークフロー上の重要判断を残す ADR または WEP
4. 完了条件を定義する 1 本の root verify コマンド
5. 正式な task runner と bootstrap 導線
6. 生成物、docs、golden fixture の明示的な規約
7. integrity と挙動回帰を分離した CI
8. hooks、plugins、repo-local skills から成る optional な agent layer

## 要点

`NEPLg2` は、文書と doctest を開発ループの中心に置く点が強いです。`plan.md`、`note.n.md`、`todo.md`、`doc/`、実行可能な docs が同期されることを前提にしています。

`vibe-lang` は、コマンド表面積の広さが強いです。`justfile` と `scripts/` が、focused gate、coverage、self-host、benchmark を CI の奥ではなくローカルから呼べる形で公開しています。

`wado` は、運用規律が最も強いです。`mise` による bootstrap、`on-task-started` / `on-task-done`、生成 docs の再生成、dirty-tree integrity check、repo-local skills が一体化しています。

そのため、このディレクトリの標準案は次を合成します。

- `NEPLg2` の明示的な計画運用と executable docs
- `vibe-lang` の豊富な focused task surface と ADR 習慣
- `wado` の環境管理、生成物 discipline、agent extension layer

## 最小導入セット

次プロジェクトでまず導入すべき最小セットは次です。

- repository boundary と完了条件を定義する `AGENTS.md`
- 短い pointer doc としての `docs/process/agent-harness.md`
- `issues/index.md`、`issues/open/`、`issues/done/`
- 最初の判断を記録する `docs/adr/ADR-0001-*.md`
- `scripts/verify-harness.sh`
- 正式なローカル task surface としての `mise.toml` または `justfile`

このディレクトリ内の他の要素は、すべてこの最小セットの上に積み上げるべきであって、置き換えるものではありません。
