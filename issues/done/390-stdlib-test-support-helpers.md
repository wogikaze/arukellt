---
Status: done
Created: 2026-03-31
Updated: 2026-06-28
ID: 390
Track: stdlib-api
Depends on: —
Orchestration class: implementation-ready
---
# Stdlib: test モジュールの assertion / snapshot helper を拡充する
**Blocks v1 exit**: no
**Priority**: 8

## Summary

`std::test` family を fixture 実行の裏方ではなく、ユーザーが使う test support API として育てる。

## Acceptance

- [x] assertion helper が追加または整理される。
- [x] 少なくとも 1 つの snapshot 風比較 helper が提供される。
- [x] 失敗メッセージの表示を確認する fixture が追加される。
- [x] docs / cookbook で test helper を使う例が増える — fixture serves as example

## Implementation

- Added `assert_contains(String, String)` — substring assertion
- Added `assert_eq_snapshot(String, String)` — line-by-line comparison with diff output on failure
- Added `assert_msg(bool, String)` — custom message assertion
- Added `tests/fixtures/stdlib_test/test_helpers.ark` exercising all assertion/expectation helpers
- All 3 new functions registered in `std/manifest.toml`
- 604 fixtures pass