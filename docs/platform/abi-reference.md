# WIT 型マッピング対応表

> **Current-first**: 現在の実装確認は [../current-state.md](../current-state.md) を参照してください。

## 概要

Arukellt が Component Model export で使用する WIT 型と、
言語型・core Wasm 表現・canonical ABI 表現の対応を示す。

## 型マッピング全 16 種

| # | WIT 型 | Arukellt 型 | core Wasm 表現 | canonical ABI | status | fixture | 備考 |
|---|--------|------------|---------------|---------------|--------|---------|------|
| 1 | `s32` | `i32` | `i32` | flat scalar | ✅ pass | `export_add.ark` | — |
| 2 | `s64` | `i64` | `i64` | flat scalar | ✅ pass | `export_i64.ark` | — |
| 3 | `float32` | `f32` | `i32` bits | flat scalar | ✅ fixture pass | `export_f32.ark` | fixture-specific bit reinterpret adapter |
| 4 | `float64` | `f64` | `f64` | flat scalar | ✅ pass | `export_f64.ark` | — |
| 5 | `bool` | `bool` | `i32` | flat scalar | ✅ pass | `export_bool.ark` | — |
| 6 | `char` | `char` | `i32` | flat scalar (Unicode) | ✅ pass | `export_char.ark` | i32 透過 |
| 7 | `tuple<…>` | `(T, U)` | linear-memory struct ptr | result-area pointer | ✅ fixture pass | `export_tuple.ark` | `tuple<s32,s32>` fixture-specific adapter |
| 8 | `enum` | `enum` (unit) | i32 discriminant | flat i32 discriminant | ✅ fixture pass | `export_enum_wit.ark` | `Color` unit enum fixture-specific path |
| 9 | `option<T>` | `Option<T>` | GC tagged ref | result-area pointer | ✅ fixture pass | `export_option.ark` | `option<s32>` fixture-specific adapter |
| 10 | `result<T,E>` | `Result<T,E>` | tagged heap object ptr | result-area pointer | ✅ fixture pass | `export_result.ark` | `result<s32,string>` fixture-specific adapter |
| 11 | `record` | `struct` | GC struct ref | flat field sequence | ✅ fixture pass | `export_record.ark` | `Point` parameter fixture-specific adapter |
| 12 | `variant` | `enum` (payload) | tagged heap object ptr | flat discriminant+payload | ✅ fixture pass | `export_variant.ark` | `Shape` f64 payload fixture-specific adapter |
| 13 | `string` | `String` | length-prefixed linear-memory ptr | `(i32, i32)` ptr+len | ✅ fixture pass | `export_string.ark` | `greet(String) -> String` fixture-specific adapter |
| 14 | `list<T>` | `Vec<T>` | linear-memory Vec header ptr | `(i32, i32)` ptr+len | ✅ fixture pass | `export_list.ark` | `list<s32>` fixture-specific adapter |
| 14 | `flags` | struct (bool fields) | GC struct ref | bitmask u32 | ❌ E0401 | `export_flags.ark` | 専用型なし; E0400 予約済み |
| 15 | `resource` | struct (handle) | GC struct ref | i32 handle index | ❌ E0401 | `export_resource.ark` | 専用型なし; E0402 予約済み |
| 16 | multi-export | 複数 `pub fn` | — | — | ✅ pass | `multi_export.ark` | 複数関数 export |

## ステータス凡例

- **✅ pass**: `component-compile:` で正常にコンポーネント生成
- **❌ E0401**: コンパイルエラー — canonical ABI 変換が未実装 (core Wasm が GC 参照型を使用)
- **W0004**: core Wasm 検証で型不整合 (GC ref 生成に起因)

## 診断コード

| コード | メッセージ | 対象 |
|--------|----------|------|
| E0400 | `WIT flags type is not supported` | flags 型 (予約) |
| E0401 | `component export uses compound/reference types not yet supported by canonical ABI` | GC ref を使う全型 |
| E0402 | `WIT resource type is not yet implemented` | resource 型 (予約) |

## 制約

### GC 参照型と canonical ABI の不整合

Arukellt の T3 バックエンドは WasmGC proposal を使用し、struct / enum / Option / Result / String / Vec を
GC 管理のヒープオブジェクト (ref 型) として表現する。Component Model の canonical ABI は
linear memory 上の flat 表現を要求するため、GC ref を直接 export することはできない。

対処方針:
1. **scalar 型** (i32, i64, f64, bool, char): canonical ABI と一致するためそのまま export 可能
2. **compound 型**: canonical ABI lift/lower adapter の実装が必要 (v3 以降)
3. **string / list**: linear memory へのコピーと `(ptr, len)` ペア化が必要

### MIR 型情報の制限

MIR のモノモーフ化により、関数シグネチャ上の struct / enum パラメータが `Type::I32` に置換される。
WIT 生成時に正確な型情報を得るには、core Wasm バイナリの型セクションを直接検査する
(`validate_core_wasm_exports`) 必要がある。

## 関連ファイル

- `src/compiler/component_wit.ark` — 型マッピング定義 (`type_to_wit`)
- `src/compiler/component.ark` — WIT 生成・検証
- `src/compiler/component_canonical_abi.ark` — canonical ABI 分類
- `tests/fixtures/component/` — fixture ファイル群
