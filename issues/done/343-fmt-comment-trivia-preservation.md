# Formatter: コメントと trivia の保存を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-06-28
**ID**: 343
**Depends on**: —
**Track**: formatter
**Blocks v1 exit**: no
**Priority**: 11

## Summary

`format_source()` が AST-based pretty-print で通常コメント・trivia・元の空行意図を完全に失う問題を解決する。

## Acceptance

- [x] 通常コメント (`//`) がフォーマット後も保持される
- [x] block comment (`/* */`) がフォーマット後も保持される
- [x] item 間の空行意図が保持される (連続する import の後に空行があれば維持)
- [x] idempotency テストがコメント付きコードで pass する

## Implementation

- Added `Comment` struct and `collect_comments()` to scan source for `//` and `/* */`
- Comments are categorized as leading or trailing by line position
- `format_source()` now collects comments and passes them to `format_module_with_comments()`
- Comments are emitted between items based on source line mapping
- `find_line_comment_start()` properly skips string literals
- Added 4 new tests: line comments between items, leading comments, block comments, idempotency
- All 32 formatter tests pass
