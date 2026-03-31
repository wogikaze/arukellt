# Formatter: コメントと trivia の保存を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 343
**Depends on**: —
**Track**: formatter
**Blocks v1 exit**: no
**Priority**: 11

## Summary

`format_source()` が AST-based pretty-print で通常コメント・trivia・元の空行意図を完全に失う問題を解決する。formatter が情報を消す限り、ユーザーが安心して `arukellt fmt` を使えない。

## Current state

- `crates/ark-parser/src/fmt.rs` (917 行): `format_module()` が AST を再印字するが、コメントは出力に含まれない
- lexer の `Token` は `LineComment` / `DocComment` を区別するが、formatter は doc comment のみ `print_doc_comments()` で扱い、通常コメントを捨てる
- block comment (`/* */`) も未対応
- 元の空行の意図 (import 後の空行、関数間の空行) が消える

## Acceptance

- [ ] 通常コメント (`//`) がフォーマット後も保持される
- [ ] block comment (`/* */`) がフォーマット後も保持される
- [ ] item 間の空行意図が保持される (連続する import の後に空行があれば維持)
- [ ] idempotency テストがコメント付きコードで pass する

## References

- `crates/ark-parser/src/fmt.rs` — formatter 実装
- `crates/ark-lexer/src/lib.rs` — Token::LineComment / DocComment
