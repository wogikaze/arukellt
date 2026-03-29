# 162: Phase 1 — Parser の Arukellt 実装

**Version**: v5 Phase 1
**Priority**: P1
**Depends on**: #161 (Lexer)

## 概要

Arukellt で書かれた再帰下降パーサーを `src/compiler/parser.ark` に実装する。Rust 版 `ark-parser` と同等の AST を生成する。

## 設計

- AST 型: struct + enum で Rust 版 AST と同等構造
- 式パーサー: Pratt parsing (演算子優先順位テーブル)
- 文パーサー: キーワード先読みで分岐
- パターンマッチ: `match expr { Pat => Expr, ... }`
- エラー: Phase 1 では最初のエラーで停止 (panic mode recovery は Phase 3)

## タスク

1. AST データ構造の定義 (Expr, Stmt, Pat, Type, FnDef, StructDef, EnumDef, ImportDecl)
2. `fn parse(tokens: Vec<Token>) -> Result<Module, ParseError>` の実装
3. Pratt parsing: 二項演算、前置演算、後置呼び出し、フィールドアクセス
4. 文の解析: let, fn, struct, enum, if, for, while, match, return, import
5. 型注釈の解析: i32, i64, f64, bool, String, Vec<T>, Option<T>, Result<T,E>, ユーザー定義型
6. Rust 版 Parser との AST 一致テスト

## 完了条件

- `arukellt compile src/compiler/parser.ark` が成功する
- 全 fixture ファイルの AST を Rust 版と比較し差分ゼロ
- 構文エラーのあるファイルで適切にエラーを報告する

## 注意事項

- AST のノードが多いため、最も使用頻度の高い構文から段階的に実装する (let, fn, if, struct → match, enum, for → import, type alias)
