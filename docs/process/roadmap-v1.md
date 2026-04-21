# v1: Wasm GC ネイティブ対応

> **状態**: **完了** (2026-03-27) — issues #019–#027 すべて `issues/done/` に移動済み
> 346/346 fixture tests pass、`verify-harness.sh` 16/16 ゲート通過。

---

## 1. 版の目的

T3 バックエンド (`wasm32-wasi-p2`) を、bridge mode (GC 型を type section に宣言するが実データは linear memory に格納する混在方式) から GC-native mode (GC 型がそのままデータを保持する純粋 GC 方式) へ完全移行する。

bridge mode の問題:
- String / Vec / Struct / Enum のデータが linear memory に存在し、ホスト GC が追跡できない
- bump allocator (`heap_ptr` グローバル) と GC が並立し、責務が不明確
- 間接呼び出しに call_indirect + Table を使用しており、GC-native の call_ref に対して非効率

---

## 2. 到達目標

1. T3 emitter が GC 型 (struct.new / array.new / br_on_cast / call_ref) のみを使用してデータを表現する
2. linear memory が WASI I/O バッファ (1 ページ固定) のみに使用される
3. `heap_ptr` グローバル、Table section、Elem section が生成バイナリから消える
4. 346 fixture tests が T3 で pass する
5. `verify-harness.sh` 16 点ゲートが全て通る

---

## 3. 対象範囲

| 対象 | 変更内容 |
|------|---------|
| `crates/ark-wasm/src/emit/t3_wasm_gc.rs` | GC-native emitter の全面書き換え |
| `crates/ark-mir/src/lower.rs` | MIR 型情報の整合確認、`is_i64_operand_mir` 等の修正 |
| `std/manifest.toml`, `std/prelude.ark` | Vec / HashMap など GC-native builtins の追加 |
| `tests/fixtures/manifest.txt` | 新規 fixture の登録 |
| `docs/current-state.md` | v1 完了状態の反映 |
| `docs/adr/ADR-002-memory-model.md` | Implementation Status 追記 |

---

## 4. 非対象範囲

- T1 (`wasm32-wasi-p1`): 一切変更しない。T3 変更が T1 テストを壊さないことを確認する。
- `--emit component`: hard error のまま維持する。v2 スコープ。
- `ark-llvm`: 変更しない。T4 は T3 の意味論に従属するが v1 では手を加えない。
- トレイト / `impl` ブロック / メソッド構文: v3–v4 スコープ。
- ネストジェネリクス (`Vec<Vec<T>>`): v3 で評価。v1 では禁止を維持。
- async/await: v5 (T5) スコープ。

---

## 5. 主要設計課題

### 5.1 Enum の subtype hierarchy

`(type $Enum.Variant (sub $Enum (struct ...)))` と `br_on_cast` を使う Option B を採用。  
Option A (flat struct + tag i32) は空間効率が悪く Wasm GC の型システムを活用しない。

Kotlin/Wasm、dart2wasm、Wasocaml が全て Option B を採用している事実を設計根拠とする。

### 5.2 String = packed i8 array

```wat
(type $string (array (mut i8)))
```

`array.new_data` で data segment から直接初期化。`array.len` で長さ取得。array は `mut i8` が必須 (`array.copy` のターゲットには mutable が要求される)。

### 5.3 Vec<T> のモノモーフィゼーション

```wat
(type $arr_i32 (array (mut i32)))
(type $vec_i32 (struct (field (mut (ref $arr_i32))) (field (mut i32))))
```

grow 時は `array.new_default` (2× サイズ) + `array.copy` + `struct.set` でバッキングアレイを差し替え。

### 5.4 closure と call_ref

現行 MIR でクロージャはキャプチャをパラメータとして渡す方式。この方式を維持しつつ、`call_indirect` (Table 依存) を `call_ref` (型付き関数参照) に置き換える。`FnRef("foo")` → `ref.func $foo`。

### 5.5 WASI I/O ブリッジ

GC string をプリントするには:
1. GC array の要素を線形メモリのスクラッチ領域 (offset 12) にコピー
2. IOV ([base=12, len]) を offset 0-7 に設定
3. `fd_write(1, &iov, 1, &nwritten)` を呼び出す

このブリッジパターンは固定 1 ページ内で完結する。

---

## 6. 実装タスク

完了済み (issues #019–#027):

| Issue | ファイル | 内容 |
|-------|---------|------|
| #019 | `t3_wasm_gc.rs` | `GcTypeRegistry` 構造体の設計、bump allocator 除去、Table/Elem section 削除 |
| #020 | `t3_wasm_gc.rs` | スカラー・制御フロー・直接 Call の検証 |
| #021 | `t3_wasm_gc.rs` | User struct → `struct.new`/`struct.get`/`struct.set` |
| #022 | `t3_wasm_gc.rs` | Enum → subtype hierarchy + `br_on_cast` |
| #023 | `t3_wasm_gc.rs` | String → `array.new_data` (GC array mut i8) |
| #024 | `t3_wasm_gc.rs` | Vec<T> → モノモーフィゼーション (`$vec_i32`, `$vec_f64`, `$vec_string`) |
| #025 | `t3_wasm_gc.rs` | Closure → `call_ref` + `ref.func`、Table/Elem 廃止 |
| #026 | `t3_wasm_gc.rs`, `std/prelude.ark` | Builtins 全実装 (to_string, parse, math, I/O, HashMap) |
| #027 | 全体 | 最終検証、clippy 修正、dead code 除去、ADR-002 更新 |

---

## 7. 検証方法

```bash
# fixture harness (346 件全 pass)
cargo test -p arukellt --test harness -- --nocapture

# unit tests (95 件)
cargo test --workspace --exclude ark-llvm

# quick verify
scripts/manager.py --quick

# full verify (16 点ゲート)
scripts/manager.py
```

---

## 8. 完了条件

| 条件 | 判定 | 結果 |
|------|------|------|
| `cargo test --workspace --exclude ark-llvm` が 0 exit | yes/no | ✅ |
| `cargo test -p arukellt --test harness` が 346/346 pass | 数値 | ✅ 346/346 |
| `scripts/manager.py` が 16/16 pass | 数値 | ✅ 16/16 |
| 生成 Wasm に `heap_ptr` グローバルが存在しない | バイナリ検査 | ✅ |
| 生成 Wasm に Table section / Elem section が存在しない | バイナリ検査 | ✅ |
| `wasmparser` validation が全 fixture で pass | validator exit code | ✅ |
| issues #019–#027 が `issues/done/` に移動済み | ファイル存在確認 | ✅ |
| `ADR-002-memory-model.md` に Implementation Status が記載 | ドキュメント確認 | ✅ |

---

## 9. 次版 (v2) への受け渡し

v2 が開始できる前提条件:

1. v1 の全完了条件が yes/✅ であること
2. `crates/ark-wasm/src/component/wit.rs` の既存 WIT 型マッピング (s32/s64/f64/bool/char/string/list/option/result/record/variant) が GC-native 型表現と矛盾しないことを確認すること
3. `GcTypeRegistry` の型インデックス割り当てが `canonical_abi.rs` の変換ロジックと整合できることを確認すること (ADR-008 の前提調査)
4. `MirModule.type_table` の `struct_defs`, `enum_defs` が WIT type mapping の拡張に十分なフィールドを持つことを確認すること

**v1 → v2 に渡す成果物**:

| 成果物 | パス |
|--------|------|
| GC-native T3 emitter | `crates/ark-wasm/src/emit/t3_wasm_gc.rs` |
| GcTypeRegistry | `t3_wasm_gc.rs` 内の `Ctx` struct |
| 型テーブル (struct/enum) | `MirModule.type_table` |
| 346 fixture baseline | `tests/fixtures/` |
| 現状ドキュメント | `docs/current-state.md` |

---

## 10. この版で特に気をつけること

1. **`is_i64_operand_mir` / `is_f64_operand_mir` の誤判定**: `parse_i64`, `parse_f64` は `Result<T, String>` を返すため i64/f64 ではない。これらを i64/f64 operand 判定に含めると emit が壊れる (`crates/ark-mir/src/lower.rs`, `crates/ark-wasm/src/emit/t1_wasm32_p1.rs`)。
2. **`struct.new` の引数順序**: Wasm GC の `struct.new $T` はフィールドを型定義の順に stack から pop する。MIR の `StructInit { fields: Vec<(String, Operand)> }` はフィールド名付きだが、emit 時に型定義順に並び替えが必要。
3. **enum base type の singleton**: `None` / `()` のような引数なし variant は `struct.new $Variant` を毎回呼ぶのではなく、global に singleton を持つとアロケーションを削減できる (v1 では任意最適化)。
4. **pre-scan の漏れ**: T3 emitter は 2 パス (pre-scan → emit)。pre-scan で検出した型情報 (extra_struct, extra_enum) が emit 時に参照されるため、新しい builtin を追加したときは必ず pre-scan 側も更新する。
5. **`#[allow(clippy::type_complexity)]` の位置**: このアトリビュートは **関数** に付ける必要がある。パラメータに付けても lint は消えない (`crates/ark-mir/src/lower.rs` で確認済み)。
6. **manifest.txt の整合**: `tests/fixtures/manifest.txt` に登録されていない `.ark` ファイルは harness が失敗する。新 fixture を追加したら必ず manifest.txt に追記すること。
7. **HashMap の線形スキャン**: v1 の `HashMap_i32_i32` は初期容量 16 の線形スキャン (O(n))。fixture のテストケースは 3 エントリのため問題ない。スケーラブルな実装は v3 スコープ。

---

## 11. この版で必ず残すドキュメント

| ドキュメント | パス | 内容 |
|------------|------|------|
| 現状ドキュメント | `docs/current-state.md` | GC-native 完了状態、型表現テーブル、バイナリサイズ比較 |
| メモリモデル ADR | `docs/adr/ADR-002-memory-model.md` | Implementation Status: GC-native 採用確定 |
| v1 ステータス | `docs/process/v1-status.md` | 全 27 issue 完了、GC-native データモデル |
| T1→T3 移行ガイド | `docs/migration/t1-to-t3.md` | bridge mode と GC-native の違い |

---

## 12. 未解決論点

1. **`rec` グループ**: 循環参照型 (linked list 等) が必要になった場合、Wasm GC の `rec { ... }` グループが必要。v1 では循環参照型を禁止しており未実装。v3 でネストジェネリクスと同時に評価する。
2. **anyref の汎用コンテナ**: 現在のジェネリック表現 (`anyref` + `ref.i31` for i32) は v1 で暫定実装。v3 のモジュール化・ADR-003 評価時に見直す。
3. **Vec の grow 戦略**: 現在 2× 固定。grow 頻度が高いユースケースでは段階的に grow 係数を変える余地がある。v4 最適化スコープ。
4. **String の immutability**: GC array は `(mut i8)` だが、Arukellt の型システムでは String は不変。ランタイム不変性を Wasm GC の immutable field で表現できないかは v4 最適化スコープ。
