# Wasm GC 実装計画

ステータス: 実装計画（決定記録ではない）
関連 ADR: ADR-035 / ADR-040
詳細設計: [RFC-007](../rfcs/007-memory64-gc-layout-and-wasi-boundary.md)

---

## ゴール

`wasm32-gc` ターゲットで既存フィクスチャスイートが全通過すること。
`wasm-tools validate --features gc` を検証ゲートとする。

## フェーズ概要

```
Phase 0: GC 命令基盤
  ↓
Phase 1: Value Representation の GC 化（MIR type system + struct/array 命令）
  ↓
Phase 2: 文字列の GC 表現
  ↓
Phase 3: Vec/Enum/Struct の GC 表現
  ↓
Phase 4: 最適化・検証・移行
```

---

## Phase 0: GC 命令基盤

- GC オペコード定義 (`opcodes.ark`)
- GC 型セクション出力 (`sections_types_gc.ark`)
- GC 命令ヘルパー (`writer_gc.ark`)
- struct/array 発行の target dispatch (`inst_struct_record.ark`, `inst_array.ark`)
- メモリセクションの条件付き削減 (`sections_memory.ark`)
- GC shape registry (`gc_shape_registry.ark`)
- Debug runner GC feature 有効化

## Phase 1: Value Representation の GC 化

**本質的な変更。MIR の値表現を `i32-as-pointer` から GC reference type に変更する。**

### 1a: MIR type system に GC reference type を追加

- `value_types.ark`: `VT_GC_REF` (値5) を定義
  - `src/compiler/corehir/value_types.ark:27-29`
  - `src/compiler/mir/value_types.ark:28-30` (ファサード)
- `MirLocal`: reference type のローカル変数をサポート
- 関数シグニチャ: パラメータ/戻り値に reference type を使用可能に

### 1b: sig_to_wasm_type で GC type を出力

- `sections_type_plan.ark`: `"ref"` sig を reference type encoding にマッピング
- 関数タイプエントリに reference type を含められるように

### 1c: struct/array 命令の Wasm 出力を修正

- `emit_struct_new` (`inst_struct_record.ark:20-45`): GC target で
  `writer_gc::emit_gc_struct_new_default` を使用
  - dest が `VT_GC_REF` の場合のみ。それ以外は線形メモリパス
- `emit_struct_get` (`inst_struct_record.ark:47-95`): GC target で
  `writer_gc::emit_gc_struct_get` を使用
- `emit_struct_set` (`inst_struct_record.ark:97-137`): 同様
- `emit_array_new` (`inst_array.ark:19-39`): GC target で
  `writer_gc::emit_gc_array_new` を使用
- `emit_array_get` (`inst_array.ark:41-65`): GC target で
  `writer_gc::emit_gc_array_get` を使用
- `emit_array_set` (`inst_array.ark:84-114`): GC target で
  `writer_gc::emit_gc_array_set` を使用

### 1d: 関数シグニチャに GC type を反映

- 関数が struct/array 参照を取る場合、Wasm シグニチャで `(ref null $T)` を使用
- `wasm-tools validate --features gc` が通ること

## Phase 2: 文字列の GC 表現

- 文字列: `(ref null (array (mut i8)))` — GC バイト配列（ADR-035 提案の layout 方針）
- `string_gc_helpers.ark`: GC array 操作の共通ヘルパー
- `intrinsic_string_basic.ark`:
  - `emit_len`: GC target で `emit_gc_array_len` を使用
  - `emit_concat`: GC target で `emit_concat_gc` を使用
  - `emit_bool_to_string`: GC target で `emit_bool_to_string_gc` を使用
  - `emit_gc_data_string`: GC array にデータをコピーするループ実装
- GC 文字列 intrinsics (`intrinsic_string_*_gc.ark`) を整備

## Phase 3: Vec/Enum/Struct の GC 表現

- `Vec<T>`: `(ref null (struct (field (mut (ref null $array_T))) (field (mut i32))))`
  — GC struct + GC array backing (capacity tracking)
  - `intrinsic_vec_push_gc.ark`, `intrinsic_vec_empty_gc.ark`,
    `intrinsic_vec_set_gc.ark`, `intrinsic_vec_pop_gc.ark` 等
- Enum: 具象 `TypeId` ごとの base + variant subtype hierarchy
  - base: discriminant field を持つ共通 supertype
  - variant: base を明示的に継承し、具象 `MirValueType` から得た型付き payload field を持つ
  - `Option` / `Result` も同じ layout 規則を使い、null や linear pointer の特例表現を作らない
  - match: tag dispatch 後にだけ `br_on_cast` / `ref.cast` で variant へ narrow する
  - `ctx_gc_enum.ark`, `ctx_gc_enum_sig.ark`, `module_gc_enum.ark`
  - `inst_gc_struct.ark`: enum variant 用 GC struct 命令
  - `match` の Wasm lowering (`br_on_cast` + `br_on_cast_fail`)
- `HashMap<K,V>`: GC struct with parallel arrays + occupancy tracking
  - `intrinsic_hashmap_str_gc.ark`
- GcLayoutTable: type section より先に module-wide plan を構築し、ADR-040 Phase 4 の
  `MirValueType -> WasmValueType` lowering spineとする
  - 同じ具象 `TypeId` の type index は一度だけ割り当てる
  - type section、signature、local、constructor、field access が同じ entry を参照する
  - 名前 prefix、固定 `gc_type_base + offset`、stack scan fallback を段階的にゼロにする

## Phase 3.5: Typed MIR / layout verifier

- Wasm body emit 前に local assignment、call signature、field access、nullability、
  lowering recipe の stack effect を検査する
- missing `TypeId` / layout は `i32` や enum-open type に fallback せず internal compiler error
- `hash_trait` のような stack underflow を invalid Wasm の validation まで遅延させない
- verifier の expected / actual には `TypeId`、repr、nullability、function / instruction を含める

## Phase 3.6: WASI P2 canonical boundary

- `HostIntrinsicSpec` に Ark-side signature と canonical boundary signature を持たせる
- pointer width は canonical memory の index type から導出する
- Memory64 から memory32 への変換が必要な場合は adapter 内で range check または copy を行う
- 通常 call site に無条件 `i32.wrap_i64` を追加しない
- pseudo core import は #714 の component-correct interface / resource adapter へ移行する
- `host_module_contract` は GC layout lane とは別に検証する

## Phase 4: 最適化・検証・移行

- t3 fixture が `wasm32-gc` target でコンパイル＆
  `wasm-tools validate --features gc` で検証
- `check-t3-wasm-validate.py` で検証ゲート
- `--target wasm32` は従来の linear memory パスを維持
- 型安全性のための WIT-level GC reference checking (W0004 gate)
- gc_hint custom section の充実 (`ctx_gc_hint.ark`, `sections_gc_hint.ark`)
- WASI P3 対応は `wasm32-gc` の `--wasi p3` フラグで切り替え（WASI P3 仕様確定後）

## 実装順序と PR 境界

1. **Verifier observe**: 10 fixture の expected / actual 型を Typed MIR verifier で再現する。
2. **Type owner**: module-wide GcLayoutTable plan を先に作り、type identity / nullability 5 件を直す。
3. **Verifier hard gate**: production fallback の利用をゼロにして missing type を hard error にする。
4. **Enum family**: enum / `Option` / `Result` constructor、match、call、return を GC subtype layout へ揃える。
5. **WASI boundary**: #714 と同じ canonical adapter spineで Memory64 pointer width を扱う。
6. **Cleanup**: name / offset / stack-scan fallback を削除し、全 gate を更新する。

同じ emitter source を変更する PR は直列化する。WASI component adapter と enum semantic
lowering は、共通の TypeId / SignatureRegistry 契約が入った後は並行に進められる。
compiler Wasm の再構築は編集をまとめて 1 回だけ行い、その成果物で対象 fixture をまとめて検証する。

---

## 検証コマンド

```bash
python3 scripts/manager.py fmt --check
python3 scripts/check/check-t3-wasm-validate.py
wasm-tools validate --features gc <output.wasm>
python3 scripts/manager.py verify component-interop
python3 scripts/manager.py verify quick
python3 scripts/manager.py selfhost fixpoint
```

emitter を変更した作業単位では、先に
`python3 scripts/manager.py selfhost build-compiler` を 1 回だけ実行する。
`selfhost fixpoint` は targeted fixture が通った後の ADR-029 gate として使い、日常の rebuild に使わない。

## リスクと依存

1. **wasmtime GC perf**: fixture parity と benchmark で監視する。
2. **Migration cost**: MIR lowering の広範な影響。段階移行が難しい場合は flag day も検討。
3. **`wasm32` / `wasm32-gc` の二重 lowering コスト**（意味論は単一、ADR-002）。
4. **Component boundary との競合**: pseudo P2 import の局所修正は #714 の最終設計と競合する。
   canonical adapter を共通 owner とし、validation 専用 truncate を入れない。

jco の Wasm GC 対応は調査で確認済み（`docs/research/target-runtime-verification.md`）。
「jco GC 待ち」は現行ブロッカーではない。

## スコープ外

- Post-MVP GC features（ADR-043）: static fields、weak references、generics
- `Weak<T>` / finalizer（言語未採択、ADR-002 / ADR-043）
- LLVM backend（ADR-045）
- WASI P3 async-first

## 関連

- [ADR-035: Wasm GC Implementation Plan](../adr/ADR-035-wasm-gc-implementation.md)
- [ADR-002: Memory Model](../adr/ADR-002-memory-model.md)
- [ADR-040: Semantic Type Spine](../adr/ADR-040-typed-mir-signature-registry.md)
- [RFC-007: Memory64 GC レイアウトと WASI P2 境界](../rfcs/007-memory64-gc-layout-and-wasi-boundary.md)
