# std/core — コアモジュール

ADR-002 により **Wasm GC 前提** で設計。
ADR-004 により **v0 ではメソッド構文なし**。すべて関数呼び出し形式。

---

## 設計方針

1. **関数呼び出しのみ**: `v.push(x)` ではなく `push(v, x)`
2. **型特化関数**: trait なしのため型ごとに提供
3. **明示的な命名**: `Vec_new_i32()`, `map_i32_i32()` など

---

## Option[T]

### 型定義

```
enum Option[T] {
    None,
    Some(T),
}
```

### 組み込み関数

```
// チェック
fn is_some[T](opt: Option[T]) -> bool
fn is_none[T](opt: Option[T]) -> bool

// 取り出し
fn unwrap[T](opt: Option[T]) -> T                      // None なら panic
fn unwrap_or[T](opt: Option[T], default: T) -> T
fn unwrap_or_else[T](opt: Option[T], f: fn() -> T) -> T

// 変換（型特化）
fn map_option_i32_i32(opt: Option[i32], f: fn(i32) -> i32) -> Option[i32]
fn map_option_i32_i64(opt: Option[i32], f: fn(i32) -> i64) -> Option[i64]
fn map_option_String_String(opt: Option[String], f: fn(String) -> String) -> Option[String]

// チェーン（型特化）
fn and_then_option_i32(opt: Option[i32], f: fn(i32) -> Option[i32]) -> Option[i32]
fn or_option_i32(a: Option[i32], b: Option[i32]) -> Option[i32]

// フィルタ（型特化）
fn filter_option_i32(opt: Option[i32], f: fn(i32) -> bool) -> Option[i32]

// Result への変換
fn ok_or[T, E](opt: Option[T], err: E) -> Result[T, E]
```

### Wasm GC 表現

- `Option[ref T]`: `(ref null $T)` — None は `ref.null`
- `Option[i32]` 等: tagged union 構造体

---

## Result[T, E]

### 型定義

```
enum Result[T, E] {
    Ok(T),
    Err(E),
}
```

### 組み込み関数

```
// チェック
fn is_ok[T, E](res: Result[T, E]) -> bool
fn is_err[T, E](res: Result[T, E]) -> bool

// 取り出し
fn unwrap[T, E](res: Result[T, E]) -> T                // Err なら panic
fn unwrap_err[T, E](res: Result[T, E>) -> E            // Ok なら panic
fn unwrap_or[T, E](res: Result[T, E], default: T) -> T
fn expect[T, E](res: Result[T, E], msg: String) -> T   // Err なら panic with msg

// 変換（型特化）
fn map_result_i32_i32[E](res: Result[i32, E], f: fn(i32) -> i32) -> Result[i32, E]
fn map_err_result[T](res: Result[T, E1], f: fn(E1) -> E2) -> Result[T, E2]

// チェーン（型特化）
fn and_then_result_i32[E](res: Result[i32, E], f: fn(i32) -> Result[i32, E]) -> Result[i32, E]

// Option への変換
fn ok[T, E](res: Result[T, E]) -> Option[T]
fn err[T, E](res: Result[T, E]) -> Option[E]
```

---

## Vec[T]

### 型定義

```
struct Vec[T] {
    // 内部実装は非公開
}
```

### 組み込み関数

```
// 作成（型特化）
fn Vec_new_i32() -> Vec[i32]
fn Vec_new_i64() -> Vec[i64]
fn Vec_new_f64() -> Vec[f64]
fn Vec_new_String() -> Vec[String]

fn Vec_with_capacity_i32(cap: i32) -> Vec[i32]
fn Vec_with_capacity_String(cap: i32) -> Vec[String]

// 基本操作
fn push[T](v: Vec[T], val: T)           // in-place
fn pop[T](v: Vec[T]) -> Option[T]
fn get[T](v: Vec[T], i: i32) -> Option[T]    // 境界チェックあり
fn get_unchecked[T](v: Vec[T], i: i32) -> T  // 境界チェックなし
fn set[T](v: Vec[T], i: i32, val: T)

// 情報
fn len[T](v: Vec[T]) -> i32
fn is_empty[T](v: Vec[T]) -> bool
fn capacity[T](v: Vec[T]) -> i32

// 変換
fn as_slice[T](v: Vec[T]) -> [T]
fn clone[T](v: Vec[T]) -> Vec[T]        // deep copy
fn clear[T](v: Vec[T])

// 高階関数（型特化）
fn map_i32_i32(v: Vec[i32], f: fn(i32) -> i32) -> Vec[i32]
fn map_i32_i64(v: Vec[i32], f: fn(i32) -> i64) -> Vec[i64]
fn map_String_String(v: Vec[String], f: fn(String) -> String) -> Vec[String]

fn filter_i32(v: Vec[i32], f: fn(i32) -> bool) -> Vec[i32]
fn filter_String(v: Vec[String], f: fn(String) -> bool) -> Vec[String]

fn fold_i32_i32(v: Vec[i32], init: i32, f: fn(i32, i32) -> i32) -> i32
fn fold_i64_i64(v: Vec[i64], init: i64, f: fn(i64, i64) -> i64) -> i64

// ソート（型特化）
fn sort_i32(v: Vec[i32])
fn sort_i64(v: Vec[i64])
fn sort_f64(v: Vec[f64])
fn sort_String(v: Vec[String])
```

### Wasm GC 表現

```wasm
(type $vec_i32 (struct
  (field $data (mut (ref null $i32_array)))
  (field $len (mut i32))
  (field $cap (mut i32))))
(type $i32_array (array (mut i32)))
```

---

## String

### 組み込み関数

```
// 作成
fn String_new() -> String
fn String_from(s: str) -> String           // リテラルから

// 基本操作
fn len(s: String) -> i32                   // バイト数
fn is_empty(s: String) -> bool
fn char_at(s: String, i: i32) -> Option[char]

// 連結（新しい String を返す）
fn concat(a: String, b: String) -> String
fn push_char(s: String, c: char) -> String  // immutable: 新規作成

// 部分文字列
fn slice(s: String, start: i32, end: i32) -> String

// 分割・結合
fn split(s: String, sep: String) -> Vec[String]
fn join(parts: Vec[String], sep: String) -> String

// 変換
fn to_bytes(s: String) -> Vec[i32]         // UTF-8 bytes
fn from_bytes(bytes: Vec[i32]) -> Result[String, StringError]

fn to_lower(s: String) -> String
fn to_upper(s: String) -> String

// 比較
fn eq(a: String, b: String) -> bool
fn starts_with(s: String, prefix: String) -> bool
fn ends_with(s: String, suffix: String) -> bool

// 複製
fn clone(s: String) -> String
```

### Wasm GC 表現

```wasm
(type $string (struct
  (field $data (ref $u8_array))
  (field $len i32)))
(type $u8_array (array (mut i8)))
```

---

## slice [T]

### 組み込み関数

```
// アクセス
fn get[T](s: [T], i: i32) -> Option[T]
fn get_unchecked[T](s: [T], i: i32) -> T

// 情報
fn len[T](s: [T]) -> i32
fn is_empty[T](s: [T]) -> bool

// 変換
fn to_vec[T](s: [T]) -> Vec[T]
```

---

## panic

```
fn panic(msg: String) -> !                // Never 型（発散）
```

プログラムを即座に終了。v0 では unwind なし（abort のみ）。

---

## 型変換

```
// 文字列 → 数値
fn parse_i32(s: String) -> Result[i32, ParseError]
fn parse_i64(s: String) -> Result[i64, ParseError]
fn parse_f64(s: String) -> Result[f64, ParseError]

// 数値 → 文字列
fn i32_to_string(n: i32) -> String
fn i64_to_string(n: i64) -> String
fn f64_to_string(n: f64) -> String
```

---

## mem（低レベル）

Wasm GC 環境での linear memory 操作。WASI 連携用。

```
// 型情報（コンパイル時定数）
fn size_of[T]() -> i32
fn align_of[T]() -> i32

// linear memory 操作
fn mem_copy(dst: i32, src: i32, n: i32)
fn mem_set(dst: i32, val: i32, n: i32)
```

---

## 使用例

### Option

```
let x: Option[i32] = Some(42)

match x {
    Some(val) => print(val),
    None => print("no value"),
}

// または
let val = unwrap_or(x, 0)
```

### Result

```
fn divide(a: f64, b: f64) -> Result[f64, String] {
    if b == 0.0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}

let result = divide(10.0, 2.0)
match result {
    Ok(val) => print(val),
    Err(e) => print(e),
}
```

### Vec

```
let v: Vec[i32] = Vec_new_i32()
push(v, 10)
push(v, 20)

let mut i = 0
while i < len(v) {
    let item = get_unchecked(v, i)
    print(item)
    i = i + 1
}

// map
fn double(x: i32) -> i32 { x * 2 }
let v2 = map_i32_i32(v, double)
```

### String

```
let s1 = String_from("hello")
let s2 = String_from(" world")
let s3 = concat(s1, s2)
print(s3)  // "hello world"

let parts = split(s3, " ")
let joined = join(parts, "-")
print(joined)  // "hello-world"
```

---

## 関連

- `docs/stdlib/cookbook.md`: 使用パターン集
- `docs/language/memory-model.md`: Wasm GC での型表現詳細
- ADR-002: メモリモデルの決定
- ADR-004: trait 導入時期の決定
