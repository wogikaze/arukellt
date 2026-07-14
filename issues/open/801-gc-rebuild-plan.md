---
Status: open
Created: 2026-07-13
Updated: 2026-07-14
ID: 801
Track: gc
Depends on: "686"
Orchestration class: blocked
Orchestration upstream: 686
Blocks v{N}: none
Priority: 2
Source: GC rebuild plan for #686 unfinished items
---

# GC 完了までのプラン

> 対象: issue #686 未完了項目、コンパイラ再ビルド、MIR 変更
> 前提: Phase 0-2 完了済み、Phase 3 Vec 完了済み

---

## 1. コンパイラ再ビルド（Fixpoint Build）

### 現状の問題

- 現在の pinned bootstrap wasm (`bootstrap/arukellt-selfhost.wasm`) が wasmtime 43.0.1 で `function[5180]: type mismatch` で動作しない
- GC ソース変更（今回追加した全ファイル）はソースコードにのみ存在し、再ビルドしないとテスト不可

### 再ビルド手順

```
Step 1: 環境確認
  wasmtime --version                          # 43.0.1 (動作確認)
  wasm-tools --version                         # 1.252.0
  python3 scripts/manager.py selfhost fixpoint # 現在の fixpoint 状態確認

Step 2: pinned wasm の動作確認
  # 現在の pinned wasm がコンパイル可能かテスト
  wasmtime run bootstrap/arukellt-selfhost.wasm -- compile --help

Step 3: 直接再ビルド
  # pinned wasm で現在のソースをコンパイル → s2
  ARUKELLT_SELFHOST_WASM=bootstrap/arukellt-selfhost.wasm \
    python3 scripts/selfhost/build.py

  # s2 が生成されたら、s2 で自己コンパイル → s3
  ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
    python3 scripts/selfhost/build.py

  # sha256 一致確認
  sha256sum .build/selfhost/arukellt-s2.wasm .build/selfhost/arukellt-s3.wasm

Step 4: fixpoint 検証 gate 通過確認
  python3 scripts/manager.py selfhost fixpoint
  python3 scripts/manager.py selfhost fixture-parity
  python3 scripts/manager.py selfhost diag-parity
```

### もし pinned wasm が動かない場合

`function[5180]` エラーは wasmtime バージョンアップ由来の可能性が高い。以下のいずれか:

| 選択肢 | 内容 | 難易度 |
|--------|------|--------|
| A | wasmtime を pinned wasm 当時のバージョンに下げる | 簡単 |
| B | wasm-tools で pinned wasm を変換/修復 | 中 |
| C | Rust レガシーコンパイラから再ビルド（利用可能なら） | 大 |
| D | 最新 wasmtime 互換の pinned wasm を上流から取得 | 中 |

---

## 2. MIR 変更: Enum GC 実装

### 現状

- `emit_struct_new`: dest local が VT_GC_REF の場合のみ GC struct.new を使用 ✅
- `emit_struct_get/set`: 常に linear-memory パスを使用（struct 型が不明なため）✅（安全側）
- Enum の variant は `lower_payload_variant_from_locals` で VT_I32 の local に生成

### アプローチ A: 新 MIR オペコード追加（推奨）

既存の MIR_STRUCT_GET/SET/NEW を変更せず、GC 専用オペコードを追加する。

```
MIR_GC_STRUCT_NEW(dest, type_idx)           — struct.new <type_idx>
MIR_GC_STRUCT_GET(base, field_idx, vt)      — struct.get <type> <field_idx>
MIR_GC_STRUCT_SET(base, value, field_idx)   — struct.set <type> <field_idx>
MIR_BR_ON_CAST(scrut, cast_type, label)     — br_on_cast <cast_type> <label>
MIR_BR_ON_CAST_FAIL(scrut, cast_type, label) — br_on_cast_fail
```

### 変更ファイル一覧

**Step 1: 新オペコード定義**

| ファイル | 変更 |
|---------|------|
| `src/compiler/mir/opcodes.ark` | `MIR_GC_STRUCT_NEW()`, `MIR_GC_STRUCT_GET()`, `MIR_GC_STRUCT_SET()`, `MIR_BR_ON_CAST()`, `MIR_BR_ON_CAST_FAIL()` 追加 |

**Step 2: インストラクションコンストラクタ**

| ファイル | 変更 |
|---------|------|
| `src/compiler/mir/inst_struct_new.ark` | `MirInst_gc_struct_new(dest, type_idx)` 追加 |
| `src/compiler/mir/inst_struct_get.ark` | `MirInst_gc_struct_get(base, field_idx, vt)` 追加 |
| `src/compiler/mir/inst_struct_set.ark` | `MirInst_gc_struct_set(base, value, field_idx, vt)` 追加 |
| 新規 | `src/compiler/mir/inst_br_on_cast.ark` — br_on_cast/br_on_cast_fail コンストラクタ |

**Step 3: MIR lowering 変更**

| ファイル | 変更内容 |
|---------|---------|
| `mir/lower/variant_payload.ark` | `lower_payload_variant_from_locals`: dest local を VT_GC_REF で生成、`MIR_GC_STRUCT_NEW` を発行 |
| `mir/lower/variant_simple.ark` | 同上（tag-only variant） |
| `mir/lower/hof_option_setup.ark` | Option/Result も GC 対応 |
| `mir/lower/aggregate_tuple.ark` | Tuple も GC 対応 |
| `mir/lower/core_match_payload_info.ark` | タグ読み取りに `MIR_GC_STRUCT_GET` を使用 |
| `mir/lower/core_match_payload_bind_core.ark` | payload 読み取りに `MIR_GC_STRUCT_GET` を使用 |
| `mir/lower/core_match_arm_branch.ark` | match dispatch に `MIR_BR_ON_CAST` / `MIR_BR_ON_CAST_FAIL` を出力 |
| `mir/lower/core_match_conditions.ark` | `core_emit_value_pattern_condition` — 型による分岐 |
| `mir/lower/struct_lit_copy.ark` | struct copy に `MIR_GC_STRUCT_GET` / `MIR_GC_STRUCT_SET` を使用 |
| `mir/lower/aggregate_field.ark` | フィールドアクセスに `MIR_GC_STRUCT_GET` / `MIR_GC_STRUCT_SET` を使用 |

**Step 4: Wasm backend 変更**

| ファイル | 変更内容 |
|---------|---------|
| `wasm/inst_dispatch_call.ark` | 新オペコードの dispatch 追加 |
| `wasm/inst_struct_record.ark` | `emit_gc_struct_new`: `MIR_GC_STRUCT_NEW` → `struct.new`<br>`emit_gc_struct_get`: `MIR_GC_STRUCT_GET` → `struct.get`<br>`emit_gc_struct_set`: `MIR_GC_STRUCT_SET` → `struct.set` |
| 新規 | `wasm/inst_br_on_cast.ark` — br_on_cast/br_on_cast_fail 発行 |
| `wasm/code_ref_locals.ark` | `MIR_GC_STRUCT_NEW` の型推論対応 |

**Step 5: Type section 変更**

| ファイル | 変更内容 |
|---------|---------|
| `wasm/sections_types_gc.ark` | Enum variant type の subtype 定義を動的生成 |
| `wasm/sections_types.ark` | Subtype 型の canon 対応は既に完了済み ✅ |

**Step 6: LowerCtx に GC flag 追加**

| ファイル | 変更内容 |
|---------|---------|
| `mir/lower/ctx_types.ark` | `LowerCtx` struct に `is_gc_target: bool` フィールド追加 |
| `mir/lower/ctx_init.ark` | `LowerCtx_new` で `is_gc_target = false` に初期化 |
| `mir/lower/ctx_api_init.ark` | setter/getter 追加 |
| `mir/lower/core_*.ark` | 必要な箇所で `ctx.is_gc_target` をチェック |

### アプローチ B: MIR オペコード変更（最小変更）

既存の `MIR_STRUCT_NEW/GET/SET` の int_val や arg0/arg1 を再利用する。

| 箇所 | 変更 |
|------|------|
| `MirInst_struct_new(dest, total_bytes)` | → `MirInst_struct_new(dest, type_idx)` に変更 |
| `MirInst_struct_get(offset, vt)` | → `MirInst_struct_get(type_idx, field_idx, vt)` に変更 |
| Wasm backend | 変更されたパラメータから GC type を直接取得 |

**デメリット**: 後方互換性なし。全 caller の一斉変更が必要。

### 推奨: アプローチ A

新オペコード追加は安全で段階的移行が可能。既存の linear-memory パスは変更不要。

---

## 3. HashMap GC

### 現状

- HashMap は stdlib で実装された高レベルデータ構造（専用 intrinsic 無し）
- Vec と struct の組み合わせで構築
- HashMap の GC 対応は **Vec GC 完了 + Enum GC 完了 + コンパイラ再ビルド** が前提

### 手順

```
1. Vec GC 完了 ✅ （済）
2. Enum/Struct GC 完了 ⏳ （MIR 変更後）
3. コンパイラ再ビルド ⏳
4. HashMap GC フィクスチャで runtime テスト
    Verify: tests/fixtures/stdlib_hashmap/* を t3-run で通過
```

---

## 4. i31ref Boxing

### 現状

- `writer_gc.ark` に `emit_gc_ref_i31` / `emit_gc_i31_get_s` は既に存在 ✅
- `opcodes.ark` に `GC_REF_I31()` / `GC_I31_GET_S()` が定義済み ✅
- 未使用（コンパイラ再ビルド後に generic コンテナで有効化予定）

### TODO

```
1. コンパイラ再ビルド ⏳
2. Vec generic で i31ref を使用する最適化パス追加
3. i31.new が出力されることの Verify
```

---

## 5. Phase 4: 検証・最適化

コンパイラ再ビルド後に以下を順次確認:

```
1. python3 scripts/manager.py verify --full
   期待: 0 failed（T1 + T3 全フィクスチャ通過）

2. python3 scripts/manager.py verify quick
   期待: 全10件の pre-existing failure が解消

3. GC 検証コマンド（#686 参照）
   arukeit compile ... --target wasm32-wasi-p2
   wasm-tools validate --features gc ...
   arukeit run ... --target wasm32-wasi-p2
```

---

## 依存関係グラフ

```
コンパイラ再ビルド (fixpoint build)
  │
  ├── Enum GC (MIR 変更) ───→ compile 通過確認
  │     │
  │     └── HashMap GC ───→ runtime テスト
  │
  ├── Phase 4 verify ─────→ 全フィクスチャ通過
  │
  └── i31ref boxing ─────→ generic 最適化
```

各タスクはコンパイラ再ビルドが解決された後に着手可能。Enum GC の MIR 変更は再ビルドと並行して設計可能だが、検証には再ビルドが必要。
