# std::bytes: Bytes、ByteBuf、ByteCursor、endian、hex、base64、leb128

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 043
**Depends on**: 039, 040
**Track**: stdlib
**Blocks v3 exit**: yes

## Summary

raw binary 処理のための std::bytes モジュールを新設する。
Bytes (不変バイト列)、ByteBuf (可変バッファ)、ByteCursor (逐次読み書き)、
endian ユーティリティ、hex/base64 エンコーディング、LEB128 可変長整数を実装する。
Wasm binary 操作と std::wasm の直接の前提。

## 背景

std.md §8 は String と Bytes を厳密に分離する設計を明記。
Wasm binary builder, canonical ABI, Component Model adapter はすべて
バイト列操作を多用する。現在の Arukellt には raw binary 型が存在しない。

## 受け入れ条件

### Bytes (不変バイト列)

```ark
pub fn bytes_new() -> Bytes
pub fn bytes_from_array(xs: Vec<u8>) -> Bytes
pub fn bytes_len(b: Bytes) -> i32
pub fn bytes_get(b: Bytes, index: i32) -> Option<u8>
pub fn bytes_slice(b: Bytes, start: i32, end: i32) -> Result<Bytes, Error>
pub fn bytes_concat(a: Bytes, b: Bytes) -> Bytes
pub fn bytes_eq(a: Bytes, b: Bytes) -> bool
```

### ByteBuf (可変バッファ)

```ark
pub fn buf_new() -> ByteBuf
pub fn buf_with_capacity(cap: i32) -> ByteBuf
pub fn buf_push_u8(buf: ByteBuf, x: u8)
pub fn buf_push_u16_le(buf: ByteBuf, x: u16)
pub fn buf_push_u32_le(buf: ByteBuf, x: u32)
pub fn buf_push_u64_le(buf: ByteBuf, x: u64)
pub fn buf_extend(buf: ByteBuf, bytes: Bytes)
pub fn buf_freeze(buf: ByteBuf) -> Bytes
pub fn buf_len(buf: ByteBuf) -> i32
```

### ByteCursor (逐次読み書き)

```ark
pub fn cursor_new(data: Bytes) -> ByteCursor
pub fn cursor_pos(c: ByteCursor) -> i32
pub fn cursor_remaining(c: ByteCursor) -> i32
pub fn read_u8(c: ByteCursor) -> Result<u8, Error>
pub fn read_u16_le(c: ByteCursor) -> Result<u16, Error>
pub fn read_u32_le(c: ByteCursor) -> Result<u32, Error>
pub fn read_u64_le(c: ByteCursor) -> Result<u64, Error>
pub fn read_u32_be(c: ByteCursor) -> Result<u32, Error>
pub fn read_bytes(c: ByteCursor, n: i32) -> Result<Bytes, Error>
```

### エンコーディング

```ark
pub fn hex_encode(b: Bytes) -> String
pub fn hex_decode(s: String) -> Result<Bytes, Error>
pub fn base64_encode(b: Bytes) -> String
pub fn base64_decode(s: String) -> Result<Bytes, Error>
pub fn leb128_encode_u32(x: u32) -> Bytes
pub fn leb128_decode_u32(c: ByteCursor) -> Result<u32, Error>
pub fn leb128_encode_i32(x: i32) -> Bytes
pub fn leb128_decode_i32(c: ByteCursor) -> Result<i32, Error>
pub fn leb128_encode_u64(x: u64) -> Bytes
pub fn leb128_decode_u64(c: ByteCursor) -> Result<u64, Error>
```

## 実装タスク

1. `ark-typecheck`: Bytes, ByteBuf, ByteCursor 型の登録
2. `ark-wasm/src/emit`: Bytes は GC array (mut u8) として表現。ByteBuf は struct {data: array, len: i32}
3. `std/bytes/bytes.ark`: Bytes 基本操作 (intrinsic)
4. `std/bytes/buf.ark`: ByteBuf 操作 (intrinsic + source)
5. `std/bytes/cursor.ark`: ByteCursor (source 実装、内部で Bytes + offset)
6. `std/bytes/endian.ark`: endian 変換 (source 実装、bit shift ベース)
7. `std/bytes/hex.ark`: hex encode/decode (source 実装)
8. `std/bytes/base64.ark`: base64 encode/decode (source 実装)
9. `std/bytes/leb128.ark`: LEB128 codec (source 実装)

## 検証方法

- fixture: `stdlib_bytes/bytes_basic.ark`, `stdlib_bytes/buf_basic.ark`,
  `stdlib_bytes/cursor_read.ark`, `stdlib_bytes/endian.ark`,
  `stdlib_bytes/hex.ark`, `stdlib_bytes/base64.ark`,
  `stdlib_bytes/leb128.ark`, `stdlib_bytes/bytes_slice.ark`
- leb128 fixture は Wasm spec の既知テストベクトルで検証

## 完了条件

- Bytes/ByteBuf/ByteCursor が GC-native Wasm で動作する
- hex/base64/leb128 のエンコード・デコードが正しい
- fixture 8 件以上 pass

## 注意点

1. Bytes の Wasm 表現: `(array mut i8)` を流用するか `(array u8)` を新設するかの判断 — u8 型 (#040) の完成度に依存
2. ByteCursor は mutable state を持つ — GC struct として pos フィールドを更新
3. LEB128 の最大バイト数制限 (u32: 5 bytes, u64: 10 bytes) を超えた場合は Error を返す

## 次版への受け渡し

- std::wasm (053) は Bytes と ByteCursor を直接使用する
- std::text (042) の `from_utf8`/`to_utf8_bytes` は Bytes 型に依存
- std::io (050) の Reader/Writer は Bytes を入出力単位とする

## ドキュメント

- `docs/stdlib/bytes-reference.md`: 全 API リファレンス + endian/leb128 の使用例

## 未解決論点

1. ByteView (読み取り専用 view) を v3 に含めるか
2. Bytes の immutability を型レベルで保証するか、convention で保証するか
3. エンディアン指定を enum (`Endian::Little`, `Endian::Big`) にするか関数名で分けるか
