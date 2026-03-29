# 163: Phase 1 — Driver + CLI の Arukellt 実装

**Version**: v5 Phase 1
**Priority**: P1
**Depends on**: #161 (Lexer), #162 (Parser)

## 概要

Arukellt で書かれた Driver と CLI エントリーポイントを実装する。Phase 1 完了時点では Lexer + Parser のみを呼び出し、AST をダンプする。

## タスク

1. `src/compiler/main.ark`: CLI エントリーポイント
   - `args()` でコマンドライン引数取得
   - サブコマンド: `parse` (AST ダンプ), `compile` (Phase 2 以降で有効化)
2. `src/compiler/driver.ark`: パイプラインオーケストレーション
   - `fs_read_file(path)` でソース読み込み
   - `tokenize()` → `parse()` の呼び出し
   - 結果の stdout 出力
3. エラーハンドリング: ファイル不在、パースエラーで exit(1)
4. `ARUKELLT_DUMP_PHASES` 環境変数対応 (Phase 1: tokens, ast)

## 完了条件

- `arukellt compile src/compiler/*.ark -o arukellt-p1.wasm` が成功する
- `wasmtime run arukellt-p1.wasm -- parse tests/fixtures/basic/hello.ark` が AST を出力する
- exit code が正常系で 0、エラー系で 1

## 注意事項

- Phase 1 Driver は compile サブコマンドを stub にしておく (Phase 2 で Resolver + TypeChecker を接続)
- args() の挙動は WASI によって異なるため、WASI P1/P2 両方でテスト
