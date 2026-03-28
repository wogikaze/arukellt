# std::test — テストユーティリティ

> **状態**: 基本 assert は実装済み。v3 で `use std::test` 明示 import + 機能拡充予定。

---

## 現行 API (prelude 経由)

```ark
// 基本アサーション (prelude 自動 import)
pub fn assert(cond: bool)
pub fn assert_eq<T>(a: T, b: T)
pub fn assert_ne<T>(a: T, b: T)
pub fn assert_eq_i64(a: i64, b: i64)
pub fn assert_eq_str(a: String, b: String)
```

---

## v3 追加 API

### アサーション

```ark
use std::test

// 真偽
pub fn assert_true(cond: bool)
pub fn assert_false(cond: bool)

// 型別等価チェック (現行の assert_eq より明示的)
pub fn assert_eq_i32(a: i32, b: i32)
pub fn assert_eq_i64(a: i64, b: i64)       // 既存
pub fn assert_eq_f64(a: f64, b: f64, eps: f64)  // 浮動小数点近似比較
pub fn assert_eq_str(a: String, b: String)      // 既存

// エラーアサーション
pub fn assert_err<T>(r: Result<T, String>)                        // Err を期待
pub fn assert_ok<T>(r: Result<T, String>) -> T                    // Ok を期待、値を返す
pub fn assert_none<T>(opt: Option<T>)
pub fn assert_some<T>(opt: Option<T>) -> T

// メッセージつきアサーション
pub fn assert_with_msg(cond: bool, msg: String)
pub fn assert_eq_with_msg<T>(a: T, b: T, msg: String)
```

### フォーマット改善

v3 では panic メッセージに値を含める:

```
// v2 (現行)
assertion failed

// v3 (予定)
assert_eq failed: left = 3, right = 4
assert_eq_str failed:
  left:  "hello"
  right: "world"
```

### スナップショットテスト (v3/v4)

```ark
// 初回実行時: expected を tests/snapshots/<name>.txt に保存
// 2回目以降: ファイルと比較
pub fn assert_snapshot(name: String, actual: String)
pub fn assert_snapshot_debug<T>(name: String, value: T)   // v4: display 実装が必要
```

### ベンチマーク (bench-lite, v3 Experimental)

```ark
// シンプルなサイクルカウンタ ベースの bench (clock_now 使用)
pub fn bench_run(name: String, iterations: i32, f: fn())
// 出力例: bench_run "fib(30)" × 1000: avg 1.23ms, min 1.10ms, max 2.01ms
```

---

## テスト構造体とフレームワーク (v4 評価)

v4 以降で検討:

```ark
// v4: #[test] アノテーション + test runner
#[test]
fn test_add() {
    assert_eq(add(1, 2), 3)
}
```

v3 ではアノテーションなし。テストは fixture ファイル (`tests/fixtures/`) で `main()` から直接 assert を呼ぶ形式を維持。

---

## 使用例

```ark
use std::test

fn test_parse_i32() {
    assert_ok(parse_i32("42"))
    assert_eq_i32(unwrap(parse_i32("42")), 42)
    assert_err(parse_i32("abc"))
    assert_err(parse_i32(""))
}

fn test_vec_sort() {
    let v = Vec_new_i32()
    push(v, 3)  push(v, 1)  push(v, 2)
    sort_i32(v)
    assert_eq_i32(get(v, 0), 1)
    assert_eq_i32(get(v, 1), 2)
    assert_eq_i32(get(v, 2), 3)
}
```

---

## v3 実装 issue

- [#045](../../issues/open/045-test-module.md) — std::test モジュール
