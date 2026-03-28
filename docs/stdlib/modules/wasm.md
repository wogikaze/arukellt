# std::wasm / std::wit / std::component — Wasm 特化 API

> **状態**: 未実装。v3/v4 で設計・実装予定。  
> Arukellt 固有の strongest differentiator。

---

## 設計方針

Arukellt はコンパイラ自身が Wasm を出力する。そのため Wasm binary utilities と WIT 型を  
「外部 crate」ではなく標準ライブラリとして提供する。

自己ホスト (v5) でのコンパイラ実装に `std::wasm` と `std::wit` が直接使われる想定。

---

## std::wasm::types

```ark
pub enum ValType {
    I32 | I64 | F32 | F64 | FuncRef | ExternRef | AnyRef | EqRef
}

pub enum RefType {
    FuncRef | ExternRef | AnyRef | EqRef | ArrayRef | StructRef
}

pub struct FuncType {
    params: Vec<ValType>,
    results: Vec<ValType>,
}

pub struct Limits {
    min: i32,
    max: Option<i32>,
}

pub enum Mutability { Const | Mut }
```

---

## std::wasm::leb128 (v3)

`std::bytes::leb128` として実装。基礎的な Wasm binary codec に必要。

→ [bytes.md](bytes.md#leb128) 参照

---

## std::wasm::binary (v3/v4)

Wasm モジュールのバイナリ読み書き。

```ark
// モジュールビルダー
pub fn module_new() -> WasmModule
pub fn module_add_type(m: WasmModule, ty: FuncType) -> i32       // type index
pub fn module_add_func(m: WasmModule, type_idx: i32) -> i32       // func index
pub fn module_add_export(m: WasmModule, name: String, kind: ExportKind, idx: i32)
pub fn module_add_import(m: WasmModule, module: String, name: String, desc: ImportDesc)
pub fn module_add_data(m: WasmModule, bytes: Bytes) -> i32         // data index
pub fn module_encode(m: WasmModule) -> Bytes                       // → wasm binary

// モジュールパーサー
pub fn module_decode(bytes: Bytes) -> Result<WasmModule, String>

pub enum ExportKind { Func | Table | Memory | Global }
```

---

## std::wasm::instr (v4)

Wasm 命令セット。コード生成で使用。

```ark
pub enum Instr {
    I32Const(i32) | I64Const(i64) | F64Const(f64)
    I32Add | I32Sub | I32Mul | I32DivS | I32RemS
    I64Add | I64Sub | I64Mul
    F64Add | F64Sub | F64Mul | F64Div
    I32Eq | I32Ne | I32LtS | I32GtS | I32LeS | I32GeS
    LocalGet(i32) | LocalSet(i32) | LocalTee(i32)
    GlobalGet(i32) | GlobalSet(i32)
    Call(i32) | CallIndirect(i32, i32)
    If(Vec<ValType>, Vec<Instr>, Vec<Instr>)
    Block(Vec<ValType>, Vec<Instr>)
    Loop(Vec<ValType>, Vec<Instr>)
    Br(i32) | BrIf(i32) | BrTable(Vec<i32>, i32)
    Return | Unreachable | Nop
    Drop | Select
    // GC 拡張
    StructNew(i32) | StructGet(i32, i32) | StructSet(i32, i32)
    ArrayNew(i32) | ArrayGet(i32) | ArraySet(i32) | ArrayLen
    RefNull(RefType) | RefIsNull | RefCast(RefType) | RefTest(RefType)
}
```

---

## std::wit (v4)

WIT 型システムを Arukellt データとして操作する。コード生成・WIT 出力に使用。

```ark
pub enum WitType {
    Bool | U8 | U16 | U32 | U64
    S8 | S16 | S32 | S64
    F32 | F64 | Char | String_
    List(Box<WitType>)
    Option_(Box<WitType>)
    Result_(Box<WitType>, Box<WitType>)
    Tuple(Vec<WitType>)
    Record(Vec<(String, WitType)>)
    Variant(Vec<(String, Option<WitType>)>)
    Enum_(Vec<String>)
    Flags(Vec<String>)
    Resource(String)
    Borrow(String) | Own(String)
}

// インターフェース定義
pub struct WitFunc {
    name: String,
    params: Vec<(String, WitType)>,
    results: Vec<WitType>,
}

pub struct WitInterface {
    name: String,
    functions: Vec<WitFunc>,
    types: Vec<(String, WitType)>,
}

pub struct WitWorld {
    name: String,
    imports: Vec<(String, WitInterface)>,
    exports: Vec<(String, WitInterface)>,
}

// 出力
pub fn wit_print_world(world: WitWorld) -> String
pub fn wit_print_interface(iface: WitInterface) -> String
pub fn wit_parse_type(s: String) -> Result<WitType, String>
```

---

## std::component (v4)

canonical ABI の lift/lower ヘルパー。Component Model グルーコードに使用。

```ark
// Resource handle 管理
pub struct ResourceHandle { id: i32 }
pub struct HandleTable<T>

pub fn handle_table_new<T>() -> HandleTable<T>
pub fn handle_table_insert<T>(t: HandleTable<T>, x: T) -> ResourceHandle
pub fn handle_table_get<T>(t: HandleTable<T>, h: ResourceHandle) -> Option<T>
pub fn handle_table_remove<T>(t: HandleTable<T>, h: ResourceHandle) -> Option<T>

// canonical ABI 変換 (GC ↔ linear memory)
pub fn lift_string(ptr: i32, len: i32) -> String
pub fn lower_string(s: String, malloc: fn(i32) -> i32) -> (i32, i32)
pub fn lift_list_i32(ptr: i32, len: i32) -> Vec<i32>
pub fn lower_list_i32(v: Vec<i32>, malloc: fn(i32) -> i32) -> (i32, i32)
```

---

## v3/v4 実装ロードマップ

| モジュール | バージョン | issue |
|-----------|----------|-------|
| `std::bytes::leb128` | v3 | [#040](../../issues/open/040-bytes-binary-stdlib.md) |
| `std::wasm::types` | v3 (型定義のみ) | [#048](../../issues/open/048-wasm-binary-utils.md) |
| `std::wasm::binary` | v4 | v4 issue |
| `std::wasm::instr` | v4 | v4 issue |
| `std::wit` | v4 | v4 issue |
| `std::component` | v4 (canonical ABI 完成後) | v4 issue |
