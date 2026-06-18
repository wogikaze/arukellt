# Wasm GC 実装計画

> 対応 ADR: [ADR-035](adr/ADR-035-wasm-gc-implementation.md)
> 追跡 issue: [#686](../issues/open/686-wasm-gc-selfhost-implementation.md)
> ステータス: Phase 0 完了, Phase 1 完了, Phase 2 部分実装中

## 全体アーキテクチャ

```
T1 (wasm32-wasi-p1): linear memory (bump allocator + i32 load/store) — 従来パス、変更なし
T2 (wasm32-freestanding): Wasm GC — struct.new/array.new など、WASI なし
T3 (wasm32-wasi-p2): Wasm GC + WASI P2 imports — メインターゲット
T5 (wasm32-wasi-p3): Wasm GC + WASI P3 imports — 将来
```

**値表現の移行**: `i32-as-pointer` → `(ref null T)` GC reference

| データ型 | 線形メモリ表現 | GC 表現 |
|---------|---------------|---------|
| i32/bool | i32 | i32 (unboxed) |
| i64 | i64 | i64 (unboxed) |
| f64 | f64 | f64 (unboxed) |
| String | length-prefixed bytes + i32 ptr | `(ref null (array (mut i8)))` |
| Vec<T> | (data_ptr, len, cap) in memory | `(ref null (struct (field (mut (ref $arr_T))) (field (mut i32))))` |
| Struct T | bump-allocated memory | `(ref null (struct ...))` |
| Enum | discriminated union in memory | subtype hierarchy + `br_on_cast` |
| HashMap | complex heap structure | struct + parallel arrays |

## Phase 0: GC 命令基盤 ✅ 完了 (2026-06-17)

### 実装済みファイル

| ファイル | 役割 |
|---------|------|
| `src/compiler/wasm/opcodes.ark` | GC オペコード定義: struct.new(0), struct.get(2), struct.set(5), array.new(6), array.new_default(7), array.get(11), array.set(14), array.len(15), array.get_u(13), ref.null(208), GC_PREFIX(251) |
| `src/compiler/wasm/writer_gc.ark` | GC 命令発行ヘルパー: emit_gc_struct_new, get/set, array_new, get/set, len, get_u |
| `src/compiler/wasm/sections_types_gc.ark` | GC 型セクション出力: A_i8, A_i32, S_f0〜S_f8 の struct/array type entries |
| `src/compiler/wasm/sections_types.ark` | emit_sig_val_type で "ref" → `(ref null $gc_type)`、"gcref" → 同 |
| `src/compiler/wasm/ctx_gc_type.ark` | GC 型インデックス管理: i32_array_type, string_type, struct_type |
| `src/compiler/wasm/gc_shape_registry.ark` | Struct/array shape 収集基盤 |
| `src/compiler/wasm/code_locals.ark` | ローカル変数宣言の GC type dispatch |
| `src/compiler/wasm/code_ref_locals.ark` | GC reference local type inference |
| `src/compiler/wasm/sections_memory.ark` | GC ターゲットは 1-page メモリ |
| `src/compiler/wasm/inst_store_policy.ark` | MIR_ARRAY_GET/SET/NEW を direct consumer に追加 |
| `tools/host-linker/src/lib.rs` | run_wasm_p2 で P2 import 自動検出・スタブ登録 |
| `tools/host-linker/src/debug_runner.rs` | P2/WASI 動的 import stub 登録 |

## Phase 1: Value Representation GC 化 ✅ 完了 (2026-06-19)

### 1a: MIR type system — VT_GC_REF

| ファイル | 変更内容 | 状態 |
|---------|---------|------|
| `corehir/value_types.ark` | VT_GC_REF = 5 追加 | ✅ |
| `mir/value_types.ark` | VT_GC_REF facade | ✅ |
| `mir/value_types.ark` | VT_GC_REF = 5 (値) | ✅ |

### 1b: Type signature GC 対応

| ファイル | 変更内容 | 状態 |
|---------|---------|------|
| `wasm/sections_type_plan.ark` | val_type_to_sig: vt==5 → "gcref", sig_to_wasm_type で "ref"/"gcref" 処理 | ✅ |
| `wasm/sections_types.ark` | emit_sig_val_type: "gcref" → WASM_REF_NULL + base+8, "ref" → WASM_REF_NULL + base | ✅ |
| `wasm/constants.ark` | WASM_REF_NULL = 99 (0x63 = ref.null prefix) | ✅ |

### 1c: Struct/Array GC 命令出力

| ファイル | 変更内容 | 状態 |
|---------|---------|------|
| `wasm/inst_struct_record.ark` | emit_struct_new/get/set: GC path で struct.new/get/set + local.set | ✅ |
| `wasm/inst_array.ark` | emit_array_new: GC path で array.new_default + local.set | ✅ |
| `wasm/inst_array.ark` | emit_array_get: GC path で array.get (ref/index はスタックから) | ✅ |
| `wasm/inst_array.ark` | emit_array_set: GC path で local.tee + local.get ref + i32.const idx + local.get val + array.set | ✅ |

### 1d: 関数シグニチャ

| ファイル | 変更内容 | 状態 |
|---------|---------|------|
| `mir/lower/params_fn.ark` | パラメータ型に VT_GC_REF を使用 | ✅ |
| `mir/lower/params_method.ark` | メソッド self パラメータに VT_GC_REF | ✅ |
| `mir/lower/return_typeinfo.ark` | 戻り値型に VT_GC_REF | ✅ |
| `mir/lower/aggregate_array.ark` | array.new/array.get/array.set で VT_GC_REF | ✅ |
| `mir/lower/struct_lit.ark` | struct.new で VT_GC_REF | ✅ |

## Phase 2: 文字列 GC 表現 🟡 部分実装中 (2026-06-19)

### 2a: GC type — A_i8

| ファイル | 変更内容 | 状態 |
|---------|---------|------|
| `wasm/sections_types_gc.ark` | A_i8 追加（base+0）、A_i32（base+1）、struct 型（base+2〜9）、count→10 | ✅ |
| `wasm/ctx_gc_type.ark` | i32_array_type → base+1, string_type → base, struct_type → base+2 | ✅ |
| `wasm/opcodes.ark` | GC_ARRAY_GET_U = 13 追加 | ✅ |
| `wasm/writer_gc.ark` | emit_gc_array_len, emit_gc_array_get_u 追加 | ✅ |

### 2b: String 操作 — len, char_at, is_empty, len_bytes ✅

| ファイル | 関数 | 変更内容 | 状態 |
|---------|------|---------|------|
| `wasm/intrinsic_string_basic.ark` | emit_len | GC: array.len; T1: i32.load(ptr-4) | ✅ |
| `wasm/intrinsic_string_access.ark` | emit_char_at | GC: array.get_u A_i8; T1: i32.load8_u(ptr+idx) | ✅ |
| `wasm/intrinsic_string_access.ark` | emit_text_is_empty | GC: array.len + eqz; T1: i32.load(ptr-4) + eqz | ✅ |
| `wasm/intrinsic_string_access.ark` | emit_text_len_bytes | GC: array.len; T1: i32.load(ptr-4) | ✅ |

### 2c: String 操作 — concat, substring, eq, slice, starts_with, ends_with

| ファイル | 関数 | 作業内容 | 状態 |
|---------|------|---------|------|
| `wasm/intrinsic_string_basic.ark` | emit_concat | GC: array.new + 要素コピー; T1: bump alloc + memory.copy | ❌ |
| `wasm/intrinsic_string_slice.ark` | emit_slice / emit_substring | GC: array.copy; T1: bump alloc + memory.copy | ❌ |
| `wasm/intrinsic_math.ark` | emit_string_eq_intrinsic | GC: array.len + 各要素 array.get_u 比較; T1: i32.load8_u | ❌ |
| `wasm/intrinsic_string_affix.ark` | emit_starts_with / emit_ends_with | GC: array.get_u; T1: i32.load8_u | ❌ |
| `wasm/intrinsic_string_chars.ark` | emit_chars | GC: 文字列イテレータの GC 表現 | ❌ |

### 2d: 定数文字列

| ファイル | 関数 | 作業内容 | 状態 |
|---------|------|---------|------|
| `wasm/inst_const.ark` | emit_const_string | GC: array.new_fixed or array.new_default + array.set で定数 bytes から GC 配列生成 | ❌ |
| `wasm/strings.ark` | prepare_string_table | 文字列テーブルの GC 対応 | ❌ |
| `wasm/sections_data.ark` | — | データセクションの文字列レイアウト（GC 時には raw bytes のみ必要） | ❌ |

### 2e: その他の文字列操作

| ファイル | 関数 | ステータス |
|---------|------|----------|
| `wasm/call_text_core.ark` | 各種文字列呼び出しディスパッチ | ❌ 各ハンドラが ctx を受け取れる必要あり |
| `wasm/call_text.ark` | 同上（slice, join, split） | ❌ |
| `wasm/call_text_extra.ark` | 同上（starts_with, trim, etc.） | ❌ |

## Phase 3: Vec/Enum/Struct の GC 表現

### 3a: Vec<T>

| ファイル | 作業内容 | 状態 |
|---------|---------|------|
| `mir/lower/aggregate_array.ark` | Vec の GC struct 表現（data_array_ref, length, capacity） | ❌ |
| `mir/lower/body_aggregate.ark` | Vec 操作の GC lowering | ❌ |
| `wasm/call_vec.ark` | Vec push/pop/len の GC 命令発行 | ❌ |

Vec GC 表現: `(ref null (struct (field (mut (ref null $array_T))) (field (mut i32))))`
- field 0: データ配列への参照
- field 1: 現在の長さ (i32)

### 3b: Enum

| ファイル | 作業内容 | 状態 |
|---------|---------|------|
| `mir/lower/enum_core.ark` | match の br_on_cast lowering | ❌ |
| `wasm/opcodes.ark` | GC_BR_ON_CAST, GC_BR_ON_CAST_FAIL 定義（既存） | ✅ |
| `wasm/call_seq.ark` | br_on_cast 発行 | ❌ |
| `mir/lower/dispatch.ark` | subtype 階層の lowering | ❌ |

Enum GC 表現:
```
base type: (struct (field (mut i32)))  — discriminant tag
variant:   (sub final (struct (field (mut i32) (field (mut $payload)))))
dispatch:  br_on_cast $base_type $variant_type
```

### 3c: Struct (既存改善)

| ファイル | 作業内容 | 状態 |
|---------|---------|------|
| `wasm/ctx_gc_type.ark` | struct 型インデックスの正確な計算（byte_size → field_count） | 🟡 改善余地 |
| `wasm/gc_shape_registry.ark` | f64/i64 フィールドの shape 対応 | ❌ |

### 3d: HashMap

| ファイル | 作業内容 | 状態 |
|---------|---------|------|
| `wasm/call_hash.ark` | HashMap 操作の GC 表現 | ❌ |

## Phase 4: 検証・最適化

### 4a: フィクスチャ

| Fixture | 内容 | ステータス |
|---------|------|----------|
| `tests/fixtures/t3/array_gc.ark` | 配列リテラル + arr[0] 参照 | ✅ t3-run: 登録済み、出力確認済み |
| `tests/fixtures/t3/string_gc.ark` | 文字列操作 (len, charAt, concat) | ❌ 追加予定 |
| `tests/fixtures/t3/struct_gc.ark` | 構造体 new/get/set | ❌ 追加予定 |
| `tests/fixtures/t3/enum_gc.ark` | enum match + br_on_cast | ❌ 追加予定 |

### 4b: 検証項目

| 項目 | 内容 | 状態 |
|------|------|------|
| wasm-tools validate --features gc | 生成 Wasm の検証 | ✅ 通過 |
| t3-run 全通過 | 既存 T3 fixture + GC fixture | 🟡 array_gc のみ追加 |
| T1 退行チェック | T1 の linear memory パスが変化していないこと | ✅ 167/167 pass |

## ファイル依存関係マップ

```
opcodes.ark
  └─ writer_gc.ark
       ├─ inst_struct_record.ark
       ├─ inst_array.ark
       └─ intrinsic_string_*.ark

constants.ark
  └─ sections_types_gc.ark
       └─ sections_types.ark → sections_type_plan.ark

ctx_gc_type.ark
  ├─ code_locals.ark
  ├─ code_ref_locals.ark
  ├─ inst_struct_record.ark
  ├─ inst_array.ark
  └─ intrinsic_string_*.ark

emit_target.ark
  ├─ sections_memory.ark
  ├─ code_locals.ark
  ├─ inst_struct_record.ark
  ├─ inst_array.ark
  └─ intrinsic_string_*.ark
```

## 実装優先順位

1. **Phase 2c**（concat, substring, eq）— 文字列操作の基本セット。これが揃うと文字列 GC が実用的に
2. **Phase 2d**（定数文字列）— 文字列リテラルの GC 配列生成。`"hello"` が動くように
3. **Phase 3a**（Vec GC）— Vec 操作の GC 化。ジェネリクス対応
4. **Phase 3b**（Enum GC）— `match` 式の `br_on_cast` lowering
5. **Phase 4**（検証・最適化）— 全フィクスチャ通過確認

## 技術的注意点

1. **string_len_from_stack**: GC パスは `array.len`（`0xfb 0x0f`）を使用。T1 は `i32.load(ptr-4)` の従来パス維持
2. **char_at**: GC パスは `array.get_u`（`0xfb 0x0d`）を使用。要素型に合わせて get/get_s/get_u を使い分け
3. **concat**: バンプアロケーションの代わりに `array.new` で新規配列＋ `array.copy` またはループで要素コピー
4. **定数文字列**: 現在はデータセクションに配置し「内容へのポインタ（offset）」を i32 として保持。GC では `array.new_fixed`（`0xfb 0x0f`）でデータ埋め込み、または `array.new_default` + ループで初期化
5. **関数シグニチャ**: 文字列参照を取る関数は `(ref null A_i8)` をパラメータ型に持つ。val_type_to_sig で VT_REF → "ref" → emit_sig_val_type で `(ref null $T)` に解決される
6. **ローカル変数**: code_ref_locals.infer_ref_local_gc_type が文字列ローカルを検出して `(ref null A_i8)` の型宣言を行う
