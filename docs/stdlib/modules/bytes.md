# std::bytes — バイナリ / エンコーディング

> **状態**: 未実装。v3 で設計・実装予定。

---

## 設計原則

`String` は UTF-8 テキスト、`Bytes` は raw binary bytes。両者は厳密に分離する。  
変換は `text::to_utf8_bytes` / `text::from_utf8` を通じて明示的に行う。

---

## Bytes (不変バイト列)

```ark
// 生成
pub fn bytes_empty() -> Bytes
pub fn bytes_from_vec(xs: Vec<u8>) -> Bytes
pub fn bytes_from_hex(s: String) -> Result<Bytes, String>
pub fn bytes_from_base64(s: String) -> Result<Bytes, String>

// 問い合わせ
pub fn bytes_len(b: Bytes) -> i32
pub fn bytes_is_empty(b: Bytes) -> bool
pub fn bytes_get(b: Bytes, i: i32) -> u8
pub fn bytes_slice(b: Bytes, start: i32, end: i32) -> Result<Bytes, String>
pub fn bytes_eq(a: Bytes, b: Bytes) -> bool

// 加工
pub fn bytes_concat(a: Bytes, b: Bytes) -> Bytes

// エンコード
pub fn bytes_to_hex(b: Bytes) -> String
pub fn bytes_to_base64(b: Bytes) -> String
```

---

## ByteBuf (可変バッファ)

文字列 builder の binary 版。生成中の bytes を効率的に積み上げる。

```ark
// 生成
pub fn buf_new() -> ByteBuf
pub fn buf_with_capacity(cap: i32) -> ByteBuf

// 追加
pub fn buf_push_u8(buf: ByteBuf, x: u8)
pub fn buf_push_u16_le(buf: ByteBuf, x: u16)
pub fn buf_push_u32_le(buf: ByteBuf, x: u32)
pub fn buf_push_u64_le(buf: ByteBuf, x: u64)
pub fn buf_push_i32_le(buf: ByteBuf, x: i32)
pub fn buf_push_i64_le(buf: ByteBuf, x: i64)
pub fn buf_extend(buf: ByteBuf, bytes: Bytes)
pub fn buf_extend_str_utf8(buf: ByteBuf, s: String)

// 完成
pub fn buf_freeze(buf: ByteBuf) -> Bytes
pub fn buf_len(buf: ByteBuf) -> i32
pub fn buf_capacity(buf: ByteBuf) -> i32
```

---

## ByteCursor (読み書きカーソル)

`Bytes` 上を位置管理しながら読み進める。バイナリ解析やフォーマット読解に使用。

```ark
// 生成
pub fn cursor_new(bytes: Bytes) -> ByteCursor
pub fn cursor_pos(c: ByteCursor) -> i32
pub fn cursor_remaining(c: ByteCursor) -> i32
pub fn cursor_is_done(c: ByteCursor) -> bool

// 読み取り (LE = little-endian, BE = big-endian)
pub fn read_u8(c: ByteCursor) -> Result<u8, String>
pub fn read_u16_le(c: ByteCursor) -> Result<u16, String>
pub fn read_u32_le(c: ByteCursor) -> Result<u32, String>
pub fn read_u64_le(c: ByteCursor) -> Result<u64, String>
pub fn read_i32_le(c: ByteCursor) -> Result<i32, String>
pub fn read_i64_le(c: ByteCursor) -> Result<i64, String>
pub fn read_bytes(c: ByteCursor, n: i32) -> Result<Bytes, String>
pub fn read_to_end(c: ByteCursor) -> Bytes

// 書き込み (書き込み可能 cursor)
pub fn write_u8(c: ByteCursor, x: u8) -> Result<(), String>
pub fn write_u32_le(c: ByteCursor, x: u32) -> Result<(), String>
pub fn write_u64_le(c: ByteCursor, x: u64) -> Result<(), String>

// スキップ・シーク
pub fn skip(c: ByteCursor, n: i32) -> Result<(), String>
pub fn seek(c: ByteCursor, pos: i32) -> Result<(), String>
```

---

## LEB128 (Wasm varint codec)

Wasm binary format で使われる可変長整数エンコーディング。

```ark
// 読み取り (ByteCursor から)
pub fn read_var_u32(c: ByteCursor) -> Result<u32, String>
pub fn read_var_u64(c: ByteCursor) -> Result<u64, String>
pub fn read_var_i32(c: ByteCursor) -> Result<i32, String>
pub fn read_var_i64(c: ByteCursor) -> Result<i64, String>

// 書き込み (ByteBuf へ)
pub fn write_var_u32(buf: ByteBuf, x: u32)
pub fn write_var_u64(buf: ByteBuf, x: u64)
pub fn write_var_i32(buf: ByteBuf, x: i32)
pub fn write_var_i64(buf: ByteBuf, x: i64)

// バイトサイズ計算 (書き込み前のサイズ見積もり)
pub fn leb128_u32_size(x: u32) -> i32
pub fn leb128_u64_size(x: u64) -> i32
```

### 使用例

```ark
let buf = buf_new()
write_var_u32(buf, 42)    // [0x2a]
write_var_u32(buf, 300)   // [0xac, 0x02]
let bytes = buf_freeze(buf)
```

---

## エンコーディングユーティリティ

```ark
// hex
pub fn hex_encode(b: Bytes) -> String
pub fn hex_decode(s: String) -> Result<Bytes, String>
pub fn hex_encode_upper(b: Bytes) -> String

// base64
pub fn base64_encode(b: Bytes) -> String
pub fn base64_decode(s: String) -> Result<Bytes, String>
pub fn base64url_encode(b: Bytes) -> String    // URL-safe variant

// endian utilities (standalone)
pub fn u32_to_le_bytes(x: u32) -> Bytes    // [b0, b1, b2, b3]
pub fn u64_to_le_bytes(x: u64) -> Bytes
pub fn le_bytes_to_u32(b: Bytes) -> Result<u32, String>
pub fn le_bytes_to_u64(b: Bytes) -> Result<u64, String>
```

---

## v3 実装ロードマップ

`Bytes` / `ByteBuf` / `ByteCursor` は GC-native (Wasm GC `(array mut i8)` / struct) で実装。  
`leb128`, `hex`, `base64` は Arukellt ソースで実装 (intrinsic 不要)。

実装 issue: [#040](../../issues/open/040-bytes-binary-stdlib.md)
