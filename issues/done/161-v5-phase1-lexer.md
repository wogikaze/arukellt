---
Depends on: "#159 (仕様凍結), #160 (stdlib チェックリスト)"
Priority: P1
Track: main
Orchestration class: implementation-ready
---

# 161: Phase 1 — Lexer の Arukellt 実装
- トークン型: "`enum Token { Ident(String), Number(i64), Float(f64), Str(String), Punct(String), Keyword(String), EOF }`"
- 入力: "`String` (ファイル全体)"
- 出力: `Vec<Token>`
- 実装方式: 文字単位ループ + match による分岐
- 位置情報: "`struct Span { start: i32, end: i32 }` を各トークンに付与"
3. `fn tokenize(source: String) -> Vec<Token>` の実装
# 161: Phase 1 — Lexer の Arukellt 実装

## 概要

Arukellt で書かれた Lexer を `src/compiler/lexer.ark` に実装する。Rust 版 `ark-lexer` と同等のトークン列を生成する。

## 設計

- トークン型: `enum Token { Ident(String), Number(i64), Float(f64), Str(String), Punct(String), Keyword(String), EOF }`
- 入力: `String` (ファイル全体)
- 出力: `Vec<Token>`
- 実装方式: 文字単位ループ + match による分岐
- 位置情報: `struct Span { start: i32, end: i32 }` を各トークンに付与

## タスク

1. Token enum の定義
2. Span struct の定義
3. `fn tokenize(source: String) -> Vec<Token>` の実装
4. キーワード判定テーブル (fn, let, if, else, match, struct, enum, import, return, for, while, break, continue, true, false, ...)
5. 文字列リテラル (エスケープシーケンス \n, \t, \\, \" 対応)
6. 数値リテラル (i32, i64, f64 の区別)
7. コメント (// 行コメント)
8. Rust 版 Lexer との出力一致テスト

## 完了条件

- `arukellt compile src/compiler/lexer.ark` が成功する
- 10 個以上のテストケースで Rust 版と同一トークン列を生成する
- `scripts/run/compare-outputs.sh lexer` で差分ゼロ

## 注意事項

- Unicode は Phase 1 では ASCII のみ対応。UTF-8 完全対応は Phase 3。
- エラー報告は最初の不正文字で停止 (エラー回復なし)。