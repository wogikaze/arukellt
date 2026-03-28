# std::text — UTF-8 文字列

> **状態**: 基本 API は実装済み (prelude 経由)。`use std::text` モジュール import は v3 で追加予定。

---

## String

Arukellt の `String` は UTF-8 エンコード済みのテキストを表す。  
raw bytes が必要な場合は `std::bytes::Bytes` を使うこと。

### 生成

```ark
pub fn String_new() -> String                           // 空文字列
pub fn String_from(lit: String) -> String               // リテラルから clone
pub fn string_repeat(s: String, n: i32) -> String       // v3: "abc" × 3 = "abcabcabc"
```

### 問い合わせ

```ark
pub fn string_len(s: String) -> i32                    // UTF-8 バイト長
pub fn string_len_chars(s: String) -> i32              // v3: Unicode スカラー値の個数
pub fn is_empty(s: String) -> bool                     // len == 0
pub fn starts_with(s: String, prefix: String) -> bool
pub fn ends_with(s: String, suffix: String) -> bool
pub fn contains(s: String, needle: String) -> bool
pub fn index_of(s: String, needle: String) -> Option<i32>   // v3: Option 返却
pub fn char_at(s: String, i: i32) -> char
```

### 変換・加工

```ark
pub fn concat(a: String, b: String) -> String
pub fn clone(s: String) -> String
pub fn slice(s: String, start: i32, end: i32) -> String
pub fn to_lower(s: String) -> String
pub fn to_upper(s: String) -> String
pub fn trim(s: String) -> String
pub fn trim_start(s: String) -> String    // v3
pub fn trim_end(s: String) -> String      // v3
pub fn replace(s: String, from: String, to: String) -> String
pub fn push_char(s: String, c: char)      // in-place append
```

### 分割・結合

```ark
pub fn split(s: String, sep: String) -> Vec<String>
pub fn split_once(s: String, sep: String) -> Option<(String, String)>  // v3
pub fn lines(s: String) -> Vec<String>    // v3: "\n" で split
pub fn join(parts: Vec<String>, sep: String) -> String
```

### エンコーディング変換 (v3)

```ark
// std::bytes との橋渡し
pub fn to_utf8_bytes(s: String) -> Bytes
pub fn from_utf8(bytes: Bytes) -> Result<String, String>
```

---

## char

```ark
pub fn char_to_string(c: char) -> String
pub fn char_to_i32(c: char) -> i32         // Unicode code point
pub fn char_from_i32(n: i32) -> Option<char>   // v3
pub fn char_is_alphabetic(c: char) -> bool     // v3
pub fn char_is_numeric(c: char) -> bool        // v3
pub fn char_is_whitespace(c: char) -> bool     // v3
```

---

## StringBuilder (v3)

文字列の断片的な構築に使用する。`concat` を多用するよりも効率的。

```ark
pub fn builder_new() -> StringBuilder
pub fn builder_push(b: StringBuilder, s: String)
pub fn builder_push_char(b: StringBuilder, c: char)
pub fn builder_push_i32(b: StringBuilder, n: i32)
pub fn builder_finish(b: StringBuilder) -> String
pub fn builder_len(b: StringBuilder) -> i32
```

### 使用例

```ark
let b = builder_new()
builder_push(b, "Hello, ")
builder_push(b, name)
builder_push_char(b, '!')
let result = builder_finish(b)
```

---

## フォーマット (v3)

```ark
// 数値フォーマット
pub fn format_i32_hex(n: i32) -> String     // "0x1a2b"
pub fn format_i32_bin(n: i32) -> String     // "0b10110"
pub fn format_i32_pad(n: i32, width: i32, pad: char) -> String
pub fn format_f64_prec(f: f64, prec: i32) -> String  // 小数点以下 prec 桁
```

---

## v3 での変更予定

- `use std::text` / `use std::text::string` 明示 import を追加
- `StringBuilder` を実装し `string_concat_builder` で効率的な連結を提供
- `lines(s)` / `trim_start` / `trim_end` / `split_once` を追加
- `from_utf8` / `to_utf8_bytes` で `std::bytes` との相互変換を整備
