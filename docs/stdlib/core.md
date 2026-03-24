# std/core — コアモジュール

ADR-002 により **Wasm GC 前提** で設計する。
ADR-004 により **v0 ではメソッド構文なし**。組み込み関数として提供。

---

## core/mem

Wasm GC 前提での低レベル操作。

```
// 型のサイズとアライメント（コンパイル時定数）
fn size_of<T>() -> i32
fn align_of<T>() -> i32

// linear memory 操作（WASI 連携用）
fn mem_copy(dst: i32, src: i32, n: i32)
fn mem_set(dst: i32, val: i32, n: i32)
```

**GC 環境での変更点**:
- `unsafe` 相当の操作は linear memory との境界のみ
- GC ヒープ上のオブジェクトは参照カウントやライフタイム管理不要

---

## core/option

```
enum Option<T> {
    None,
    Some(T),
}

// 組み込み関数として提供
fn is_some<T>(opt: Option<T>) -> bool
fn is_none<T>(opt: Option<T>) -> bool
fn unwrap<T>(opt: Option<T>) -> T                      // None なら panic
fn unwrap_or<T>(opt: Option<T>, default: T) -> T
fn unwrap_or_else<T>(opt: Option<T>, f: fn() -> T) -> T

// 型特化版（trait なし）
fn option_i32_map(opt: Option<i32>, f: fn(i32) -> i32) -> Option<i32>
fn option_i32_and_then(opt: Option<i32>, f: fn(i32) -> Option<i32>) -> Option<i32>
fn option_i32_filter(opt: Option<i32>, f: fn(i32) -> bool) -> Option<i32>
```

**Wasm GC での表現**:
- `Option<ref T>`: null 許容参照 `(ref null $T)` — None は `ref.null`
- `Option<i32>` 等: tagged union 構造体

---

## core/result

```
enum Result<T, E> {
    Ok(T),
    Err(E),
}

// 組み込み関数として提供
fn is_ok<T, E>(res: Result<T, E>) -> bool
fn is_err<T, E>(res: Result<T, E>) -> bool
fn unwrap<T, E>(res: Result<T, E>) -> T                // Err なら panic
fn unwrap_err<T, E>(res: Result<T, E>) -> E            // Ok なら panic
fn unwrap_or<T, E>(res: Result<T, E>, default: T) -> T

// 型特化版（trait なし）
fn result_i32_map(res: Result<i32, E>, f: fn(i32) -> i32) -> Result<i32, E>
fn result_i32_and_then(res: Result<i32, E>, f: fn(i32) -> Result<i32, E>) -> Result<i32, E>
```

---

## collections/string

**Wasm GC 前提の設計**

```wasm
;; Wasm GC 表現
(type $string (struct
  (field $data (ref $u8_array))
  (field $len i32)))
(type $u8_array (array (mut i8)))
```

前提:
- UTF-8 エンコード
- **不変（immutable）** — 変更には新しい String を作成
- GC ヒープ上に配置

API（組み込み関数）:
```
fn string_new() -> String
fn len(s: String) -> i32            // バイト数
fn is_empty(s: String) -> bool
fn string_to_bytes(s: String) -> [i32]  // UTF-8 バイト列

// 連結（新しい String を返す）
fn concat(a: String, b: String) -> String
fn string_append_char(s: String, c: char) -> String  // 新しい String を返す（不変）
```

**v0 での制限**:
- 文字列フォーマット（`format!` 相当）は trait（Display）が必要なため後回し
- `+` 演算子による連結は trait 導入後

---

## collections/vec

**Wasm GC 前提の設計**

```wasm
;; Wasm GC 表現
(type $vec_T (struct
  (field $data (mut (ref null $T_array)))
  (field $len (mut i32))
  (field $cap (mut i32))))
(type $T_array (array (mut T)))
```

前提:
- 可変長配列
- GC ヒープ上に確保
- capacity 超過時に配列を grow（古い配列は GC が回収）
- `[T]`（スライス）はビュー

API（組み込み関数）:
```
fn vec_new<T>() -> Vec<T>
fn vec_with_capacity<T>(cap: i32) -> Vec<T>
fn len<T>(v: Vec<T>) -> i32
fn is_empty<T>(v: Vec<T>) -> bool
fn vec_capacity<T>(v: Vec<T>) -> i32
fn vec_push<T>(v: Vec<T>, val: T)           // in-place 変更
fn vec_pop<T>(v: Vec<T>) -> Option<T>
fn vec_get<T>(v: Vec<T>, i: i32) -> Option<T>    // 境界チェックあり
fn vec_get_unchecked<T>(v: Vec<T>, i: i32) -> T  // 境界チェックなし
fn as_slice<T>(v: Vec<T>) -> [T]
fn vec_clear<T>(v: Vec<T>)
```

**v0 での制限**:
- `sort`, `dedup` 等は Ord 相当の制約が必要 → trait 導入後
- イテレーション（`for` 構文）は trait 導入後

---

## 関連

- `docs/language/memory-model.md`: Wasm GC での型表現詳細
- ADR-002: メモリモデルの決定
- ADR-004: trait 導入時期の決定
