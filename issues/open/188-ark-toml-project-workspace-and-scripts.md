# `ark.toml`: project / workspace metadata と `script run` surface

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 188
**Depends on**: 124
**Track**: parallel
**Blocks v1 exit**: no

## Summary

`issues/open/124-wit-component-import-syntax.md` では `ark.toml` が WIT / component import 用の最小 manifest として構想されているが、VS Code の all-in-one 体験に必要なのはそれだけではない。
project root 検出、target / emit の既定値、workspace 単位の扱い、`[scripts]` 定義、`script run` / `script list` の安定 CLI がないと、拡張は tasks や quick pick を shell 個別実装で継ぎはぎすることになる。

本 issue では、まだ full package manager には踏み込まず、IDE と CLI が共有できる「プロジェクトマニフェスト」としての `ark.toml` を定義する。

## 受け入れ条件

1. `ark.toml` が少なくとも `package`、任意の `workspace`、`scripts`、tool defaults を表現できる
2. `arukellt script run <name>` が project root を自動検出し、`[scripts]` の named command を実行できる
3. `arukellt script list --json` が IDE 向けに scripts 一覧を machine-readable に返せる
4. args passthrough、env、cwd、exit code、unknown script 時の error message が安定している
5. `ark.toml` がない単体ファイル利用と、manifest-aware project 利用の両方を壊さない
6. docs / examples / fixtures が追加され、VS Code task provider から利用可能である

## 実装タスク

1. #124 の manifest parser 設計を一般 project metadata に拡張する
2. `scripts` schema と CLI command surface を定義する
3. project root / workspace discovery を実装する
4. `script list` の machine-readable output を定義する
5. target / emit / wit / component などの project defaults と衝突しない形に整理する
6. docs と sample `ark.toml` を追加する

## 参照

- `issues/open/124-wit-component-import-syntax.md`
- `crates/arukellt/src/main.rs`
- `docs/current-state.md`
- `docs/contributing.md`
