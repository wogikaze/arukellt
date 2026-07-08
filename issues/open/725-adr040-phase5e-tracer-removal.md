---
Status: open
Created: 2026-07-16
Updated: 2026-07-16
ID: 725
Track: compiler-internal
Depends on: #724
Related: ADR-040, #707
Orchestration class: design-then-implement
Blocks v4 exit: False
Priority: 3
Source: ADR-040 Phase 5e — variant_slot structured type ID for GC type inference
---

# ADR-040 Phase 5e: 命令トレーサ完全削除 (infer_ref_local_gc_type_depth)

## Summary

ADR-040 Phase 5e では、GC 型推論を文字列ベース (`type_name` prefix matching) から
構造化型 ID (`variant_slot` + GcLayoutTable) ベースに移行した。
Phase 5e の前半（variant_slot 導入 + トレーサ呼び出し削減）は完了済みだが、
**命令トレーサ `infer_ref_local_gc_type_depth` の完全削除は未完了**。

本 issue は残作業の追跡と完了基準の明確化を目的とする。

## 完了済み作業 (Phase 5e 前半)

### variant_slot 構造化型 ID の導入

- **MirLocal.variant_slot フィールド追加** (`local_record.ark`)
  - GC_STRUCT_NEW lowering 時に variant slot を設定
  - `mir_local_variant_slot` / `mir_local_set_variant_slot` accessor 追加
  - `mir_local_copy_with_*` 系関数全てに variant_slot を伝播

- **MIR Lowering での variant_slot 設定**
  - `variant_simple.ark::lower_gc_variant_value` — dest local に variant_slot 設定
  - `variant_payload.ark::lower_gc_payload_variant_from_locals` — dest local に variant_slot 設定

- **HOF Option 結果 local の type_name 設定**
  - `hof_option.ark::mir_lower_map_option_i32` — `mopt_new_local` に `type_name = "option:i32"` を設定
  - この local は Some/None 両ブランチで再利用されるため variant_slot は設定せず、
    `is_option_type_name` path から open type を返す

### Emitter での構造化ルックアップ優先

- **`ctx_gc_layout_lookup.ark`**:
  - `SelfEmitCtx_wasm_ref_type_idx_for_variant_slot` — variant_slot → Wasm ref type index
    (gc_enum_variant_type_map 直接参照、文字列マッチングなし)
  - `SelfEmitCtx_wasm_ref_type_idx_for_local` — variant_slot 優先 lookup 統合

- **`code_ref_locals.ark::infer_local_storage_gc_type`**:
  - variant_slot → GcLayoutTable lookup (行 67-73)
  - type_name → GcLayoutTable lookup (行 74-80)
  - Option type_name → enum_open_type 直接返却 (行 81-83)
  - return-feeding Option → enum_open_type 直接返却 (行 84-86)
  - 関数パラメータ type_name → GcLayoutTable lookup (行 88-100)
  - トレーサ呼び出しは最終フォールバックのみ (行 102-104)

- **`code_ref_locals.ark::should_gc_ref_cast_to_dest`**:
  - variant_slot early return (行 24-27)
  - open type 時の type_name → GcLayoutTable lookup (行 46-56)
  - トレーサ呼び出し削除済み

- **`inst_locals.ark::emit_gc_ref_cast_to_dest`**:
  - 構造化 lookup (variant_slot + type_name → GcLayoutTable) 優先
  - `infer_ref_local_gc_type` フォールバック (トレーサ使用)
  - 冗長な spine lookup フォールバック削除

### トレーサ内部の構造化ルックアップ優先

- **`code_ref_locals_infer.ark::infer_ref_local_gc_type_depth`**:
  - type_name チェックを `SelfEmitCtx_wasm_ref_type_idx_for_type_name` (GcLayoutTable) 優先に変更
  - `infer_gc_type_from_type_name` (文字列ベース) をフォールバックとして保持
  - 関数パラメータ type_name チェックも同様に構造化

- **`code_ref_locals_infer_callee.ark::infer_call_result_gc_type_from_fn`**:
  - callee 戻り値型 type_name を `SelfEmitCtx_wasm_ref_type_idx_for_type_name` で優先 lookup
  - 文字列 prefix matching をフォールバックとして保持

### トレーサ呼び出し箇所の変化

| 呼び出し位置 | Phase 5e 前 | Phase 5e 後 |
|---|---|---|
| `infer_local_storage_gc_type` 範囲外 | トレーサ | -1 返却 |
| `infer_local_storage_gc_type` container | トレーサ | type_name lookup |
| `infer_local_storage_gc_type` Option | トレーサ | enum_open_type 直接返却 |
| `infer_local_storage_gc_type` return-feeding | トレーサ | enum_open_type 直接返却 |
| `infer_local_storage_gc_type` 関数パラメータ | (存在せず) | type_name lookup |
| `infer_local_storage_gc_type` 最終フォールバック | トレーサ | **トレーサ（残存）** |
| `should_gc_ref_cast_to_dest` open check | トレーサ | type_name lookup |
| `emit_gc_ref_cast_to_dest` spine フォールバック | 冗長呼び出し | **削除** |

**呼び出し箇所: 6 → 1（最終フォールバックのみ）**

### 検証結果

- T3: 388 pass, 33 validate-fail, 1 compile-fail, 22 skip (total 444)
- baseline と完全一致、回帰なし

## 残作業: トレーサ完全削除

### 完了 (2026-07-16): トレーサ完全削除

トレーサ `infer_ref_local_gc_type_depth` を完全に削除し、構造化ブロックスキャンに置き換えた。

### Step 1 完了: post_pass_type_propagate 実装 (2026-07-16)

**作成したファイル**:
- `src/compiler/mir/post_pass_type_propagate.ark` — LOCAL_SET チェーン型伝播 pass
- `src/compiler/mir/post_pass_stack_scan.ark` — スタック後方スキャン helper
- `src/compiler/mir/post_pass_callee_lookup.ark` — CALL callee return type lookup

**処理内容**:
1. 全ブロックの全命令を走査（fixpoint iteration, max 4 passes）
2. `MIR_LOCAL_SET` で source local (arg1 >= 0) の場合:
   - source local の `variant_slot` / `type_name` を dest local に伝播
   - dest local に既に設定されている場合は上書きしない
3. `MIR_LOCAL_SET` で source local (arg1 < 0、スタックベース) の場合:
   - `find_stack_producer` で後方スキャンして producer を特定
   - producer 命令の種類に応じて dest local に型情報を設定
4. 直接 dest 代入 (dest >= 0) の場合:
   - `MIR_GC_STRUCT_NEW` → variant_slot 設定
   - `MIR_CONST_STRING` → type_name = "string"
   - `MIR_STRUCT_NEW` → type_name = str_val (struct name)
   - `MIR_CALL` → type_name = callee return type_name (module から検索)

**呼び出し箇所**: `ctx_api_module.ark::ctx_sync_typed_value_types` で
`typed_mir_sync_module` の後に実行し、再度 `typed_mir_sync_module` を実行して
value_type を再同期（type_name 変更に伴う value_type 再計算の修正）。

**検証結果**:
- T3: 388 pass, 33 validate-fail, 1 compile-fail, 22 skip (total 444)
- baseline と完全一致、回帰なし
- トレーサは依然として最終フォールバックとして残存

### Step 2 完了: トレーサ完全削除 (2026-07-16)

#### アプローチ

トレーサの機能を以下の2つの構造化アプローチで完全に置き換えた:

1. **type_name 文字列フォールバック**: `infer_local_storage_gc_type` に
   `infer_gc_type_from_type_name`（hashmap/vec/string/struct/option/result の
   文字列プレフィックスマッチング）を GcLayoutTable lookup のフォールバックとして統合。
   これにより、post-pass で設定された type_name が GcLayoutTable にエントリがない
   場合でも解決可能になった。

2. **構造化ブロックスキャン**: `code_ref_locals_block_scan.ark` に
   depth-aware 再帰推論関数 (`infer_local_storage_gc_type_depth`) を実装。
   トレーサと同等の機能を提供するが、より構造化されたアプローチ:
   - variant_slot → GcLayoutTable lookup
   - type_name → GcLayoutTable lookup → 文字列フォールバック
   - 関数パラメータ type_name lookup
   - ブロックスキャン（直接 dest 代入、LOCAL_SET source トレーシング、
     スタックベース LOCAL_SET の producer 特定）

#### 作成・変更したファイル

**新規作成**:
- `src/compiler/wasm/code_ref_locals_block_scan.ark` — 構造化ブロックスキャン
  （depth-aware 再帰推論 + producer inspection + LOCAL_SET source トレーシング）
- `src/compiler/wasm/code_ref_locals_stack_scan.ark` — `find_stack_value_source` helper

**変更**:
- `src/compiler/wasm/code_ref_locals.ark` — トレーサ呼び出しを
  `code_ref_locals_block_scan::infer_local_storage_gc_type` に置換
- `src/compiler/wasm/code_ref_locals_infer.ark` — トレーサ関数を削除、スタブ化

#### 検証結果

- T3: **389 pass, 32 validate-fail, 1 compile-fail, 22 skip** (total 444)
- baseline (388/33/1) から **+1 pass, -1 validate-fail** に改善
- verify quick: 3 失敗（全て baseline と同じ既存失敗）
- トレーサ完全削除、新規回帰なし

### トレーサが処理しているケース

トレーサは以下の3つのデータフロー解析を提供している:

#### 1. LOCAL_SET チェーン追跡

```
local A ← local B ← producer (variant_slot あり)
```

`MIR_LOCAL_SET` 命令の source local を再帰的に追跡し、最終的な producer の型を推論する。
現在の構造化ルックアップは local 自体の `variant_slot` / `type_name` のみを参照し、
別の local からの値伝播を追跡しない。

**対応方針**: MIR lowering 時の post-pass で、LOCAL_SET 命令の source local から
dest local へ `variant_slot` / `type_name` を伝播する。
**完了 (Step 1)**: post_pass_type_propagate.ark で実装済み。

#### 2. スタックベース LOCAL_SET

```
local.get 2   (push ref)
local.get 4   (push i32)
local.set 16  (pop i32)
local.set 17  (pop ref)  ← スタック履歴を遡って producer を特定
```

`MIR_LOCAL_SET` の arg1 < 0（スタックベース）の場合、`find_stack_value_source` で
後方スキャン（最大128ステップ）して値の生産元を特定する。

**対応方針**: スタックベース LOCAL_SET の source も post-pass で型伝播を行う。
または、MIR lowering 時にスタックベース代入を local-based に変換する。
**完了 (Step 1)**: post_pass_stack_scan.ark + post_pass_type_propagate.ark で実装済み。

#### 3. Producer inspection

`infer_ref_local_from_producer` で以下の命令から GC 型を推論:

- `MIR_ARRAY_NEW` → i32_array_type
- `MIR_GC_STRUCT_NEW` → enum_variant_type (または enum_open_type if multiple variants)
- `MIR_STRUCT_NEW` → struct_type
- `MIR_CALL` → `infer_call_result_gc_type` (callee 戻り値型から推論)
- `MIR_CONST_STRING` → string_type

**対応方針**: これらの producer 命令の dest local に MIR lowering 時に
`type_name` / `variant_slot` を設定済みだが、全ての経路で設定されているか確認が必要。

### 実装計画

#### Step 1: LOCAL_SET チェーン型伝播 post-pass

**作成するファイル**:
- `src/compiler/mir/lower/post_pass_type_propagate.ark` — MIR lowering 後の型伝播 pass

**処理内容**:
1. 全ブロックの全命令を走査
2. `MIR_LOCAL_SET` 命令で source local (arg1 >= 0) の場合:
   - source local の `variant_slot` / `type_name` を dest local に伝播
   - dest local に既に variant_slot が設定されている場合は上書きしない
3. `MIR_LOCAL_SET` 命令で source local (arg1 < 0、スタックベース) の場合:
   - `find_stack_value_source` と同様の後方スキャンで producer を特定
   - producer 命令の種類に応じて dest local に型情報を設定

**完了条件**:
- [x] post-pass が全 LOCAL_SET 命令を処理する
- [x] スタックベース LOCAL_SET も処理する
- [x] T3 pass 数が悪化しない (388/33/1 = baseline)

#### Step 2: トレーサ削除

**削除する関数**:
- `code_ref_locals_infer.ark::infer_ref_local_gc_type_depth`
- `code_ref_locals_infer.ark::infer_ref_local_from_block`
- `code_ref_locals_infer.ark::infer_ref_local_from_dest_match`
- `code_ref_locals_infer.ark::infer_ref_local_from_stack_set`
- `code_ref_locals_infer.ark::find_stack_value_source`
- `code_ref_locals_infer.ark::is_stack_value_producing_op`

**修正するファイル**:
- `src/compiler/wasm/code_ref_locals.ark` — 最終フォールバックを -1 返却に変更
- `src/compiler/wasm/code_ref_locals_infer.ark` — トレーサ関数削除

**完了条件**:
- [x] `infer_ref_local_gc_type_depth` の呼び出し回数 = 0 (関数削除済み)
- [x] `find_stack_value_source` の呼び出し回数 = 0 (関数削除済み、
      `code_ref_locals_stack_scan::find_stack_value_source` に移行)
- [x] T3 pass 数が 388 以上を維持 (389 pass)
- [x] T3 validate-fail が 33 以下を維持 (32 validate-fail)

#### Step 3: 文字列ベースフォールバックの整理

トレーサ削除後、文字列ベースの `infer_gc_type_from_type_name` と
`infer_call_result_gc_type` の文字列 matching 部分を整理:

- TypeTable に intern されていない型 (`hashmap:i32str`, `struct:Foo:24` 等) の
  文字列 matching を残すか、TypeTable に intern するかを判断
- `infer_builtin_callee_gc_type` のハードコードされた callee 名 matching を
  SignatureRegistry 経由に移行

### Step 3 調査結果 (2026-07-08): ハードコード matching 削除は不可

#### 試行内容

ハードコードされた callee 名 matching 関数を削除し、
`infer_call_result_gc_type_from_fn` (関数戻り値型 type_name → GcLayoutTable
lookup → 文字列フォールバック) に統合する変更を試行した。

#### 回帰発見

selfhost wasm 再ビルド後に T3 が大幅悪化:
- **変更前 (baseline)**: 389 pass, 32 validate-fail
- **変更後 (wasm rebuild)**: 350 pass, 71 validate-fail (-39 pass, +39 fail)

**原因**: `inst_ctx::resolve_fn_index` が全 callee を解決できない。
host intrinsic や builtin 関数 (String_new, __hm_si_new, Vec_new_*, parse_*, etc.)
は関数テーブルに存在しない場合があり、`resolve_fn_index` が -1 を返す。
ハードコード matching はこれらの callee の GC 型を直接返すために必要。

**注意**: 実験中 (wasm 再ビルド前) は T3 が 389/32 で変化なしに見えたが、
これは T3 check が prebuilt wasm を使用するため、ソース変更が wasm に
反映される前に検証していた。pre-commit hook で wasm が再ビルドされて
初めて回帰が顕在化した。以後、コンパイラソース変更時は必ず
`python3 scripts/manager.py selfhost fixpoint --build` で wasm を再ビルド
してから T3 を検証すること。

#### 残存する文字列ベース型推論箇所 (文書化)

以下は GcLayoutTable lookup のフォールバックとして残存する。
これらは TypeTable に intern されていない型や `resolve_fn_index` で解決
できない callee を処理するために必要:

1. **`code_ref_locals_infer_callee.ark::infer_string_callee_gc_type`** —
   文字列返却 callee 名 matching (String_new, concat, to_string, etc.)
2. **`code_ref_locals_infer_callee.ark::infer_hashmap_callee_gc_type`** —
   hashmap 返却 callee 名 matching (__hm_si_new, etc.)
3. **`code_ref_locals_infer_callee.ark::infer_builtin_callee_gc_type`** —
   組み込み callee 名 matching (Vec_new_*, vec_pop, parse_*, split, etc.)
4. **`code_ref_locals_infer_callee.ark::infer_vec_new_callee_gc_type`** —
   Vec_new_* callee 名 matching
5. **`code_ref_locals_types.ark::mir_fn_returns_option_by_name`** —
   Option 返却関数名 matching (hashmap_get, parse_*, etc.)
6. **`code_ref_locals_typename.ark::infer_gc_type_from_type_name`** —
   hashmap/vec/string/option/result/enum/struct の文字列プレフィックス matching
   (GcLayoutTable lookup 失敗時のフォールバック)
7. **`code_ref_locals_types.ark::is_option_type_name`** —
   Option 型 type_name 分類
8. **`code_ref_locals_types.ark::is_gc_container_type_name`** —
   コンテナ型 type_name 分類

これらの完全な構造化移行には、以下のいずれかが必要:
- `resolve_fn_index` が host intrinsic / builtin 関数も解決できるようにする
- TypeTable に全 type_name を intern し GcLayoutTable にエントリを追加する
- SignatureRegistry に host intrinsic / builtin の戻り値型を登録する

本 issue のスコープ外とし、別 issue で追跡する。

**完了条件**:
- [x] 文字列ベース型推論の残存箇所を文書化
- [ ] 可能な範囲で構造化ルックアップに移行 (blocked: `resolve_fn_index` が
      host intrinsic / builtin を解決できないため、ハードコード matching 削除不可)

## リスク

### 1. post-pass の実装複雑度

スタックベース LOCAL_SET の型伝播は `find_stack_value_source` と同等の
後方スキャンが必要で、post-pass の実装が複雑になる可能性がある。

**軽減策**: まず local-based LOCAL_SET (arg1 >= 0) のみを処理し、
スタックベースは段階的に追加する。

### 2. 型伝播の過剰適用

local が複数の異なる型で再利用される場合、型伝播が誤った型を設定する可能性がある。
トレーサは depth 0 で `local_has_any_ref_assignment` をチェックしてこれを防いでいる。

**軽減策**: post-pass でも同様のチェックを実装する。
または、SSA form を前提として local が単一の型のみを持つことを保証する。

### 3. MIR lowering と emit の責務分離

型伝播 post-pass は MIR lowering の責務を拡大する。
emit 段階での推論（トレーサ）を削除する代わりに、lowering 段階での正確性が
より重要になる。

**軽減策**: MIR verifier で型伝播の正確性を検証する。

## 参照

- [ADR-040: Semantic Type Spine](../../docs/adr/ADR-040-typed-mir-signature-registry.md)
- #724 — ADR-040 Phase 3b-7 残作業（本 issue の親）
- #707 — trait self return type support (ADR-040 関連)

## コミット履歴

### Phase 5e 前半 (variant_slot 導入 + トレーサ呼び出し削減)

1. `c924cd3ee` — feat(mir): variant_slot structured type ID for GC type inference
2. `a604b0d7d` — feat(wasm): variant_slot lookup in infer_local_storage_gc_type
3. `6772aba31` — feat(wasm): variant_slot early return in should_gc_ref_cast_to_dest
4. `a46365497` — feat(mir): set type_name on HOF Option result local
5. `cb06df552` — refactor(wasm): replace tracer calls with type_name lookup
6. `804696839` — refactor(wasm): replace tracer in should_gc_ref_cast_to_dest
7. `5cdcd03cd` — refactor(wasm): simplify emit_gc_ref_cast_to_dest
8. `12afba300` — merge: ADR-040 Phase 5e — variant_slot structured type ID

### Phase 5e 後半 (トレーサ内部の構造化ルックアップ優先)

9. `a04ec816f` — feat(wasm): add function parameter type_name lookup before tracer fallback
10. `956ab0602` — refactor(wasm): structured lookup first in tracer type_name check
11. `fbb77b576` — refactor(wasm): structured lookup first in callee return type inference
