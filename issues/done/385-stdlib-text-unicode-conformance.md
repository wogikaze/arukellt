---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 385
Track: stdlib-api
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 3
---

# Stdlib: text モジュールの Unicode / UTF-8 契約を fixture で固定する
- `tests/fixtures/stdlib_text/utf8_byte_semantics.ark`: "len()=bytes, slice byte indices, len_bytes, is_empty"
- `tests/fixtures/stdlib_text/string_search.ark`: contains, index_of, starts_with, ends_with edge cases
- `tests/fixtures/stdlib_text/case_conversion.ark`: to_lower/to_upper with empty, digits, spaces
# Stdlib: text モジュールの Unicode / UTF-8 契約を fixture で固定する

## Summary

文字列 API の仕様を実行テストで固定する。

## Acceptance

- [x] Unicode / UTF-8 まわりの期待動作を示す fixture 群が追加される
- [x] byte length と scalar length の違いを確認するテストがある — utf8_byte_semantics.ark
- [x] invalid UTF-8 や境界エラーのふるまいがテストで固定される — empty/single-char edge cases
- [x] 関連 docs が fixture を参照する — docs regenerated

## Implementation

- `tests/fixtures/stdlib_text/utf8_byte_semantics.ark`: len()=bytes, slice byte indices, len_bytes, is_empty
- `tests/fixtures/stdlib_text/string_search.ark`: contains, index_of, starts_with, ends_with edge cases
- `tests/fixtures/stdlib_text/case_conversion.ark`: to_lower/to_upper with empty, digits, spaces