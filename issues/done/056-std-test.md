---
Status: done
Created: 2026-03-28
Updated: 2026-04-14
ID: 46
Track: stdlib
Depends on: 039, 041
Orchestration class: implementation-ready
Blocks v3 exit: yes
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---

# std: ":test: assert、snapshot テスト、bench-lite"
テスト支援ライブラリを std: ":test として実装する。"
pub fn assert_eq_i32(actual: "i32, expected: i32)"
pub fn assert_eq_i64(actual: "i64, expected: i64)"
pub fn assert_eq_f64(actual: "f64, expected: f64)"
pub fn assert_eq_string(actual: "String, expected: String)"
pub fn assert_eq_bool(actual: "bool, expected: bool)"
pub fn assert_ne_i32(actual: "i32, unexpected: i32)"
pub fn assert_ne_string(actual: "String, unexpected: String)"
pub fn assert_true(cond: bool)
pub fn assert_false(cond: bool)
pub fn expect_ok_i32(r: Result<i32, String>) -> i32  // panic if Err
pub fn expect_err_string(r: Result<i32, String>) -> String  // panic if Ok
pub fn expect_some_i32(o: Option<i32>) -> i32  // panic if None
pub fn expect_none_i32(o: Option<i32>)  // panic if Some
pub fn snapshot(name: "String, actual: String)"
// 初回実行時: 期待値ファイルを生成
// 再実行時: 期待値と actual を比較し、差異があれば panic + diff 出力
pub fn bench(name: "String, iterations: i32, f: fn() -> ())"
1. `std/test/assert.ark`: "assert_eq/assert_ne 系 (source 実装、panic ベース)"
2. `std/test/expect.ark`: expect_ok/expect_err/expect_some/expect_none
3. `std/test/snapshot.ark`: "snapshot テスト (fs_read_file/fs_write_file 使用)"
4. `std/test/bench.ark`: "簡易ベンチマーク (monotonic clock 使用)"
5. 失敗時のエラーメッセージ: `expected X, got Y` 形式で出力
- fixture: `stdlib_test/assert_eq.ark`, `stdlib_test/assert_ne.ark`,
3. bench の精度: monotonic clock が ns 解像度でなければ ms で表示
- `docs/stdlib/test-reference.md`: テスト関数一覧、snapshot 使い方、bench パターン
# std::test: assert、snapshot テスト、bench-lite

---

## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/056-std-test.md` — incorrect directory for an open issue.


## Summary

テスト支援ライブラリを std::test として実装する。
assert_eq/assert_ne (型付き比較)、expect_err (Result 検証)、
snapshot テスト、bench-lite (簡易ベンチマーク) を提供する。
stdlib 自身のテストと、ユーザーコードのテストの両方に使用。

## 受け入れ条件

### アサーション

```ark
pub fn assert_eq_i32(actual: i32, expected: i32)
pub fn assert_eq_i64(actual: i64, expected: i64)
pub fn assert_eq_f64(actual: f64, expected: f64)
pub fn assert_eq_string(actual: String, expected: String)
pub fn assert_eq_bool(actual: bool, expected: bool)
pub fn assert_ne_i32(actual: i32, unexpected: i32)
pub fn assert_ne_string(actual: String, unexpected: String)
pub fn assert_true(cond: bool)
pub fn assert_false(cond: bool)
```

### Result/Option 検証

```ark
pub fn expect_ok_i32(r: Result<i32, String>) -> i32  // panic if Err
pub fn expect_err_string(r: Result<i32, String>) -> String  // panic if Ok
pub fn expect_some_i32(o: Option<i32>) -> i32  // panic if None
pub fn expect_none_i32(o: Option<i32>)  // panic if Some
```

### snapshot テスト

```ark
pub fn snapshot(name: String, actual: String)
// 初回実行時: 期待値ファイルを生成
// 再実行時: 期待値と actual を比較し、差異があれば panic + diff 出力
```

### bench-lite

```ark
pub fn bench(name: String, iterations: i32, f: fn() -> ())
// iterations 回実行し、平均・最小・最大時間を stdout に出力
```

## 実装タスク

1. `std/test/assert.ark`: assert_eq/assert_ne 系 (source 実装、panic ベース)
2. `std/test/expect.ark`: expect_ok/expect_err/expect_some/expect_none
3. `std/test/snapshot.ark`: snapshot テスト (fs_read_file/fs_write_file 使用)
4. `std/test/bench.ark`: 簡易ベンチマーク (monotonic clock 使用)
5. 失敗時のエラーメッセージ: `expected X, got Y` 形式で出力
6. 既存 `assert` (bool のみ) との共存

## 検証方法

- fixture: `stdlib_test/assert_eq.ark`, `stdlib_test/assert_ne.ark`,
  `stdlib_test/expect_ok.ark`, `stdlib_test/expect_err.ark`,
  `stdlib_test/expect_some.ark`, `stdlib_test/bench_basic.ark`
- snapshot は `stdlib_test/snapshot_basic.ark` + `.expected` ファイルで検証

## 完了条件

- assert_eq が型付きで動作し、失敗時に informative なメッセージを出す
- expect_ok/err/some/none が正しく panic する
- bench が iterations 回実行し時間を出力する
- fixture 6 件以上 pass

## 注意点

1. assert 系はジェネリクスなしのモノモーフ版を先に入れる — ジェネリック assert_eq は v4
2. snapshot テストは fs に依存 — T1 では compile-time に warning を出す
3. bench の精度: monotonic clock が ns 解像度でなければ ms で表示

## ドキュメント

- `docs/stdlib/test-reference.md`: テスト関数一覧、snapshot 使い方、bench パターン

## 未解決論点

1. test runner (テスト関数の自動検出・実行) を v3 に入れるか
2. property-based testing (QuickCheck 的) を v3 スコープに含めるか
3. assert_eq のジェネリック版 (Display/Debug trait が必要) の扱い