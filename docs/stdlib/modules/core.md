# std::core — 基礎型と基礎関数

> **状態**: prelude として実装済み。v3 で `use std::core` 明示 import に移行予定。

---

## Option<T>

```ark
Option<T>  ::=  Some(T) | None

// コンストラクタ (prelude 自動 import)
Some(x: T) -> Option<T>
None        -> Option<T>

// 問い合わせ
pub fn is_some<T>(opt: Option<T>) -> bool
pub fn is_none<T>(opt: Option<T>) -> bool

// 取り出し
pub fn unwrap<T>(opt: Option<T>) -> T                        // panic on None
pub fn unwrap_or<T>(opt: Option<T>, default: T) -> T
pub fn unwrap_or_else<T>(opt: Option<T>, f: fn() -> T) -> T

// 変換
pub fn map_option_i32_i32(opt: Option<i32>, f: fn(i32) -> i32) -> Option<i32>
// v3: 汎用 map<T, U>(opt: Option<T>, f: fn(T) -> U) -> Option<U>

// Result への変換
pub fn ok_or<T, E>(opt: Option<T>, err: E) -> Result<T, E>
```

---

## Result<T, E>

```ark
Result<T, E>  ::=  Ok(T) | Err(E)

// コンストラクタ (prelude 自動 import)
Ok(x: T)   -> Result<T, E>
Err(e: E)  -> Result<T, E>

// 問い合わせ
pub fn is_ok<T, E>(r: Result<T, E>) -> bool
pub fn is_err<T, E>(r: Result<T, E>) -> bool

// 取り出し
pub fn unwrap<T, E>(r: Result<T, E>) -> T                          // panic on Err
pub fn unwrap_or<T, E>(r: Result<T, E>, default: T) -> T
pub fn expect<T, E>(r: Result<T, E>, msg: String) -> T            // panic with msg on Err
pub fn unwrap_err<T, E>(r: Result<T, E>) -> E

// 変換
pub fn ok<T, E>(r: Result<T, E>) -> Option<T>
pub fn err<T, E>(r: Result<T, E>) -> Option<E>
pub fn map_result_i32_i32(r: Result<i32, String>, f: fn(i32) -> i32) -> Result<i32, String>
// v3: 汎用 map<T, U, E>(r: Result<T, E>, f: fn(T) -> U) -> Result<U, E>
```

---

## 数学・比較

```ark
// 算術 (i32)
pub fn abs(n: i32) -> i32
pub fn min(a: i32, b: i32) -> i32
pub fn max(a: i32, b: i32) -> i32
pub fn clamp_i32(v: i32, lo: i32, hi: i32) -> i32

// 浮動小数点
pub fn sqrt(x: f64) -> f64
pub fn floor(x: f64) -> f64    // v3
pub fn ceil(x: f64) -> f64     // v3
pub fn round(x: f64) -> f64    // v3
pub fn pow_i32(base: i32, exp: i32) -> i32  // v3
```

---

## 型変換

```ark
// 数値 → String
pub fn i32_to_string(n: i32) -> String
pub fn i64_to_string(n: i64) -> String
pub fn f64_to_string(f: f64) -> String
pub fn bool_to_string(b: bool) -> String
pub fn char_to_string(c: char) -> String

// String → 数値 (Result 返却)
pub fn parse_i32(s: String) -> Result<i32, String>
pub fn parse_i64(s: String) -> Result<i64, String>
pub fn parse_f64(s: String) -> Result<f64, String>
pub fn parse_bool(s: String) -> Result<bool, String>   // v3
```

---

## panic / assert

```ark
pub fn panic(msg: String) -> Never

// prelude から自動 import
pub fn assert(cond: bool)
pub fn assert_eq<T>(a: T, b: T)
pub fn assert_ne<T>(a: T, b: T)
pub fn assert_eq_i64(a: i64, b: i64)
pub fn assert_eq_str(a: String, b: String)
```

---

## 出力

```ark
pub fn println(s: String)
pub fn print(s: String)
pub fn eprintln(s: String)    // stderr
```

---

## v3 での変更予定

- `use std::core` の明示 import で全 API に到達可能にする
- `Option<T>` / `Result<T,E>` の汎用 `map`, `flat_map`, `and_then` を実装
- `Error` 型を `String` より構造化した代替として追加 (`std::core::Error`)
- 全関数に stability ラベル (`Stable` / `Experimental`) を付与
