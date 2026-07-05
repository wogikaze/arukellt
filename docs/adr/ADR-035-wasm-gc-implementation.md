# ADR-035: Wasm GC Implementation Plan

ステータス: **DRAFT** — 段階実装中

決定日: 2026-06-17

---

## 文脈

ADR-002 (Memory Model, 2026-03-25) は **選択肢 A: Wasm GC 前提** を採用した。Rust
プロトタイプ (`crates/ark-wasm/src/emit/t3_wasm_gc/`) は実際に GC 命令を出力し、
542 テストが通過した。selfhost 移行 (2026-03-29) 以降は線形メモリ + bump アロケータ
を使用していたが、Phase 0-3 の実装により GC target (T2/T3) では GC 命令基盤、
GC struct/array、文字列/Vec の GC 表現が順次実装されている。T1 は線形メモリを維持。

ADR-007 (Targets) は以下のメモリモデルを定義している：

| ターゲット | メモリモデル | 現状 |
|------------|-------------|------|
| `wasm32` | Linear memory | ✅ 実装済み、wabt/iwasm 互換 |
| `wasm32-gc` | **Linear memory + Wasm GC** | 🟡 Phase 1 部分実装（GC struct/array 命令基盤、compile/validate slice、P2 imports） |
| `native-cpp` / `native-llvm` | LLVM/C++ 依存 | scaffold のみ |

selfhost エミッタには GC 命令基盤 (`writer_gc.ark`、`sections_types_gc.ark`、
`ctx_gc_type.ark`) と struct/array 発行の target dispatch が追加された。
2026-06-18 の Phase 1 スライスで、GC ターゲットは `i32` aggregate lowering
shape に対して reference local/type encoding と `struct.*` / `array.*` 命令を
出力し、基本的な array/struct fixture は `wasm-tools validate --features gc`
を通る。MIR/CoreHIR には aggregate reference locals/params/returns 用の
`VT_GC_REF` tag が追加され、GC 型は function signatures より前に emitted される。
top-level `i32`-field struct params/returns は `(ref null ...)` signature として
validate する。完全な shape registry、f64/i64 field shapes、method/component
aggregate ABI、String/Vec/Enum/Option/Result の GC 表現は未完了。

## 決定

Wasm GC 実装を以下の **5 Phase** で段階的に行う。完了基準は `wasm32-gc`
で既存フィクスチャスイートが全通過すること。

### Phase 0: GC 命令基盤 ✅ (完了, 2026-06-17)

- GC オペコード定義 (`opcodes.ark`)
- GC 型セクション出力 (`sections_types_gc.ark`)
- GC 命令ヘルパー (`writer_gc.ark`)
- struct/array 発行の target dispatch (`inst_struct_record.ark`, `inst_array.ark`)
- メモリセクションの条件付き削減 (`sections_memory.ark`)
- GC shape registry (`gc_shape_registry.ark`)
- Debug runner GC feature 有効化

### Phase 1: Value Representation の GC 化

**本質的な変更。MIR の値表現を `i32-as-pointer` から GC reference type に変更する。**

#### 1a: MIR type system に GC reference type を追加

- `value_types.ark`: 新しい `VT_GC_REF` (または既存の `VT_REF` を GC 意味論に変更)
- `MirLocal`: reference type のローカル変数をサポート
- 関数シグニチャ: パラメータ/戻り値に reference type を使用可能に

#### 1b: sig_to_wasm_type で GC type を出力

- `sections_type_plan.ark`: `"ref"` sig を `WASM_I32()` ではなく
  reference type encoding (例: `(ref null $array_i32_type)`) にマッピング
- 関数タイプエントリに reference type を含められるようにする

#### 1c: struct/array 命令の Wasm 出力を修正

- `emit_struct_new`: 現在は bump アロケータ (`GLOBAL_GET` + `I32_ADD` + `GLOBAL_SET`)、
  GC mode では `struct.new type_idx` を発行（引数はフィールド初期値）
- `emit_struct_get`: `I32_LOAD` ではなく `struct.get type_idx.field_idx`
- `emit_struct_set`: `I32_STORE` ではなく `struct.set type_idx.field_idx`
- `emit_array_new`: bump allocation ではなく `array.new type_idx count`
- `emit_array_get/set`: 同様

#### 1d: 関数シグニチャに GC type を反映

- 関数が struct/array 参照を取る場合、Wasm シグニチャで `(ref null $T)` を使用
- これにより Wasm バリデータが type check を通す

### Phase 2: 文字列の GC 表現

- 文字列: `(ref null (array (mut i8)))` — GC バイト配列（ADR-002 合意済み）
- 文字列結合 (`concat`)、部分文字列 (`substring`)、文字アクセス (`char_at`) の
  Wasm 命令を GC 配列操作に変更
- 従来の線形メモリ上の length-prefixed 文字列からの移行

### Phase 3: Vec/Enum/Struct の GC 表現

- `Vec<T>`: `(ref null (struct (field (mut (ref null $array_T))) (field (mut i32))))`
  — GC struct + GC array backing (capacity tracking)
- Enum: subtype hierarchy + `br_on_cast` dispatch（ADR-002 合意済み）
  - 基本型: `(struct (field (mut i32)))` (discriminant tag)
  - variant: `(sub final (struct (field (mut i32) (field (mut $payload)))))`
  - `match` の Wasm lowering: `br_on_cast` + `br_on_cast_fail`
- `HashMap<K,V>`: GC struct with parallel arrays + occupancy tracking

### Phase 4: 最適化・検証・移行

- `--target wasm32-gc` で既存フィクスチャスイート全通過を確認
- `--target wasm32` は従来の linear memory パスを維持
- 型安全性のための WIT-level GC reference checking (W0004 gate)
- gc_hint custom section の充実（GC type layout metadata for runtime）
- WASI P3 対応は `wasm32-gc` の `--wasi p3` フラグで切り替え（WASI P3 仕様確定後）

## スコープ外

- Post-MVP GC features (ADR-008 survey): static fields, weak references,
  generics — これらは v5 以降
- jco/javy interop (#036, #037): jco の Wasm GC サポート待ち
- LLVM backend (`native-llvm`): native target は別トラック
- WASI P3 async-first: WASI P3 仕様未確定のため defer

## リスク

1. **jco GC 非対応**: JS interop パスがブロックされる。影響範囲: browser target。
   upstream jco が Wasm GC をサポートするまで待つ必要がある (#037)。
2. **wasmtime GC perf**: GC ランタイムの最適化度合い。wasmtime 46.x では
   copying collector がデフォルトになり、DRC のバグ (29.x で問題となった) は
   回避済み。fixture parity と benchmark で監視する。
3. **Migration cost**: `i32-as-pointer` から GC reference への移行で
   MIR lowering の多くの箇所に影響。段階的移行が難しい場合、一度に切り替える
   「flag day」アプローチも検討。
4. **`wasm32` / `wasm32-gc` の二重実装コスト**: ADR-002 で「両対応」は拒否されたが、
   `wasm32` 維持のため linear memory パスは残す。コードベースが複雑になる。

## タイムライン（目標）

| Phase | 内容 | 目標完了 |
|-------|------|---------|
| 0 | GC 命令基盤 | ✅ 2026-06-17 |
| 1 | Value representation GC 化 | 2026-07 |
| 2 | 文字列 GC 表現 | 2026-07 |
| 3 | Vec/Enum/Struct GC 表現 | 2026-08 |
| 4 | 検証・最適化 | 2026-08 |

## 関連 ADR

- [ADR-002: Memory Model](ADR-002-memory-model.md) — GC-native 決定の根拠
- [ADR-007: Targets](ADR-007-targets.md) — ターゲット定義 (T1-T5)
- [ADR-008: Wasm GC Post-MVP](ADR-008-wasm-gc-post-mvp.md) — Post-MVP survey
- [ADR-013: Primary Target](ADR-013-primary-target.md) — T3 primary 根拠
