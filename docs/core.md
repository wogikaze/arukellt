# std/core — コアモジュール

ADR-002 により **Wasm GC 前提** で設計する。

---

## core/mem

Wasm GC 前提での低レベル操作。

```
// 型のサイズとアライメント（コンパイル時定数）
fn size_of[T]() -> usize
fn align_of[T]() -> usize

// 値のコピー（値型は暗黙コピー、参照型は参照コピー）
// GC 環境では明示的な copy 関数は基本不要

// linear memory 操作（WASI 連携用）
fn mem_copy(dst: *mut u8, src: *const u8, n: usize)
fn mem_set(dst: *mut u8, val: u8, n: usize)
```

**GC 環境での変更点**:
- `unsafe` 相当の操作は linear memory との境界のみ
- GC ヒープ上のオブジェクトは参照カウントやライフタイム管理不要

---

## core/option

```
enum Option[T] {
    None,
    Some(T),
}

impl Option[T] {
    fn is_some(self) -> bool
    fn is_none(self) -> bool
    fn unwrap(self) -> T                          // None なら panic
    fn unwrap_or(self, default: T) -> T
    fn unwrap_or_else(self, f: fn() -> T) -> T
    fn map[U](self, f: fn(T) -> U) -> Option[U]
    fn and_then[U](self, f: fn(T) -> Option[U]) -> Option[U]
    fn or(self, other: Option[T]) -> Option[T]
    fn ok_or[E](self, err: E) -> Result[T, E]
    fn filter(self, f: fn(T) -> bool) -> Option[T]
}
```

**Wasm GC での表現**:
- `Option[ref T]`: null 許容参照 `(ref null $T)` — None は `ref.null`
- `Option[i32]` 等: tagged union 構造体

---

## core/result

```
enum Result[T, E] {
    Ok(T),
    Err(E),
}

impl Result[T, E] {
    fn is_ok(self) -> bool
    fn is_err(self) -> bool
    fn unwrap(self) -> T                          // Err なら panic
    fn unwrap_err(self) -> E                      // Ok なら panic
    fn unwrap_or(self, default: T) -> T
    fn map[U](self, f: fn(T) -> U) -> Result[U, E]
    fn map_err[F](self, f: fn(E) -> F) -> Result[T, F]
    fn and_then[U](self, f: fn(T) -> Result[U, E]) -> Result[U, E]
    fn or[F](self, other: Result[T, F]) -> Result[T, F]
    fn ok(self) -> Option[T]
    fn err(self) -> Option[E]
}
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
- `str` は String のビュー（実体は同じ参照）

API:
```
fn String::new() -> String
fn String::from(s: str) -> String
fn String::len(self) -> usize           // バイト数
fn String::is_empty(self) -> bool
fn String::as_str(self) -> str
fn String::to_bytes(self) -> [u8]       // UTF-8 バイト列

// 連結（新しい String を返す）
fn String::concat(self, other: str) -> String
fn String::push_char(self, c: char) -> String  // 新しい String を返す
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

API:
```
fn Vec[T]::new() -> Vec[T]
fn Vec[T]::with_capacity(cap: usize) -> Vec[T]
fn Vec[T]::len(self) -> usize
fn Vec[T]::is_empty(self) -> bool
fn Vec[T]::capacity(self) -> usize
fn Vec[T]::push(self, val: T)                   // in-place 変更
fn Vec[T]::pop(self) -> Option[T]
fn Vec[T]::get(self, i: usize) -> Option[T]     // 境界チェックあり
fn Vec[T]::get_unchecked(self, i: usize) -> T   // 境界チェックなし
fn Vec[T]::as_slice(self) -> [T]
fn Vec[T]::clear(self)
```

**v0 での制限**:
- `sort`, `dedup` 等は Ord 相当の制約が必要 → trait 導入後
- イテレーション（`for` 構文）は trait 導入後

---

## 関連

- `docs/language/memory-model.md`: Wasm GC での型表現詳細
- ADR-002: メモリモデルの決定
- ADR-004: trait 導入時期の決定
