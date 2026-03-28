# std::wasm: Wasm バイナリ型・opcode・module builder

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 053
**Depends on**: 039, 040, 043
**Track**: stdlib
**Blocks v3 exit**: no (Experimental)

## Summary

Wasm バイナリの型定義 (ValType, FuncType, Limits)、opcode enum、
module builder、binary reader/writer を Arukellt の std::wasm として実装する。
Wasm ツールチェーン、自己ホスト、component glue 生成の基盤。

## 安定性ラベル

**Experimental** — Wasm spec 更新に追随して API が変わる可能性がある。

## 受け入れ条件

### std::wasm::types

```ark
pub enum ValType { I32, I64, F32, F64, V128, FuncRef, ExternRef }
pub enum NumType { I32, I64, F32, F64 }
pub enum RefType { FuncRef, ExternRef }
pub struct FuncType { params: Vec<ValType>, results: Vec<ValType> }
pub struct Limits { min: u32, max: Option<u32> }
pub struct MemoryType { limits: Limits }
pub struct TableType { element: RefType, limits: Limits }
pub struct GlobalType { val_type: ValType, mutable: bool }
```

### std::wasm::binary (module builder)

```ark
pub fn module_builder() -> WasmModuleBuilder
pub fn add_type(m: WasmModuleBuilder, ty: FuncType) -> u32
pub fn add_import_func(m: WasmModuleBuilder, module: String, name: String, type_idx: u32) -> u32
pub fn add_func(m: WasmModuleBuilder, type_idx: u32, locals: Vec<ValType>, body: Vec<u8>) -> u32
pub fn add_memory(m: WasmModuleBuilder, mem: MemoryType) -> u32
pub fn add_export_func(m: WasmModuleBuilder, name: String, func_idx: u32)
pub fn add_export_memory(m: WasmModuleBuilder, name: String, mem_idx: u32)
pub fn encode(m: WasmModuleBuilder) -> Bytes
pub fn decode(bytes: Bytes) -> Result<WasmModule, Error>
```

### std::wasm::leb128

```ark
pub fn encode_u32(x: u32) -> Bytes
pub fn decode_u32(c: ByteCursor) -> Result<u32, Error>
pub fn encode_i32(x: i32) -> Bytes
pub fn decode_i32(c: ByteCursor) -> Result<i32, Error>
pub fn encode_u64(x: u64) -> Bytes
pub fn decode_u64(c: ByteCursor) -> Result<u64, Error>
pub fn size_u32(x: u32) -> i32  // encoded byte count
```

## 実装タスク

1. `std/wasm/types.ark`: ValType, FuncType 等の enum/struct 定義
2. `std/wasm/binary.ark`: WasmModuleBuilder (source 実装、ByteBuf ベース)
3. `std/wasm/leb128.ark`: LEB128 codec (source 実装)
4. `std/wasm/decode.ark`: 基本的な Wasm binary parser (section-level)
5. `std/wasm/opcode.ark`: 主要命令の定数定義 (i32.const = 0x41 等)
6. Wasm spec のマジックナンバー (0x00 0x61 0x73 0x6D) とバージョン検証

## 検証方法

- fixture: `stdlib_wasm/valtype.ark`, `stdlib_wasm/leb128_roundtrip.ark`,
  `stdlib_wasm/module_builder.ark`, `stdlib_wasm/encode_minimal.ark`,
  `stdlib_wasm/decode_header.ark`, `stdlib_wasm/opcode_basic.ark`
- 生成した Wasm binary が wasmparser で valid であることを検証

## 完了条件

- module_builder で最小限の Wasm module を生成し、バイナリが valid
- LEB128 roundtrip が正しい
- fixture 6 件以上 pass

## 注意点

1. module builder は Wasm 1.0 (MVP) を対象 — GC proposal 拡張は v4
2. opcode 定数は全命令を網羅する必要はない — 主要命令 (i32.const, call, local.get 等) から
3. decode は section レベルの分割のみ — instruction-level disassembly は v4

## ドキュメント

- `docs/stdlib/wasm-reference.md`: types, binary, leb128, opcode のリファレンス

## 未解決論点

1. GC proposal の型 (struct, array, ref) を v3 に含めるか
2. module builder の streaming emit (大きなモジュールで ByteBuf が肥大化するリスク)
