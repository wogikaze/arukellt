# Test UX: `arukellt test` と VS Code Test Explorer surface

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 186
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

`std::test` 自体は存在するが、`issues/done/056-std-test.md` にも未解決論点として残っている通り、test runner（テスト関数の自動検出・実行）はまだ first-class ではない。
VS Code の all-in-one 拡張で Test Explorer を成立させるには、言語ランタイム / CLI 側に「テストとは何か」「どう列挙し、どう実行し、どう結果を返すか」という安定 surface が必要である。

本 issue では、Arukellt のテスト宣言・検出・実行・machine-readable reporting を CLI surface として固め、拡張が shell の生ログ解析に依存しなくて済む状態を作る。

## 受け入れ条件

1. テスト関数 / テストファイル / テストモジュールの discovery rule が定義され、docs から辿れる
2. `arukellt test` で package / file / single test name 単位の実行ができる
3. `arukellt test --json` などの machine-readable reporter が、discover / start / pass / fail / skip / duration / location を出力する
4. assertion failure / snapshot mismatch / panic が test result と source location に結び付く
5. 失敗テストの再実行、filter 実行、snapshot 更新フローを CLI surface として扱える
6. VS Code Test Explorer がこの surface をそのまま利用できる

## 実装タスク

1. test declaration / discovery rule を決める
2. runner と reporter 形式（JSON lines など）を定義する
3. `arukellt test` subcommand を実装する
4. `std::test` の assertion / snapshot failure を runner と接続する
5. fixtures / docs / sample project を追加する

## 参照

- `issues/done/056-std-test.md`
- `docs/cookbook/testing-patterns.md`
- `docs/stdlib/modules/test.md`
- `docs/current-state.md`
