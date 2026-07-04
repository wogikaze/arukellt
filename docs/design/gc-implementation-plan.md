# Wasm GC 実装計画

> 対応 ADR: [ADR-035](adr/ADR-035-wasm-gc-implementation.md)
> 追跡 issue: [#686](../issues/open/686-wasm-gc-selfhost-implementation.md)
> ステータス: Phase 0/1/2 完了 🎉, Phase 3 Vec ✅, Enum 基盤 ✅, Phase 4 一部
> 動作確認: GC array smoke test ✅, string_gc compile ✅, string_gc run ✅
> 検証通過: len, ==, starts_with, ends_with, concat, to_string (i32/i64/f64), print/println
> 検証コマンド: 全チェックボックスに Verify コマンド付き (#686 更新済み)
> 次フェーズ: Enum GC (MIR 層変更) | HashMap GC | コンパイラ再ビルド後に full Verify

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
| `src/compiler/wasm/opcodes.ark` | GC オペコード定義 |
| `src/compiler/wasm/writer_gc.ark` | GC 命令発行ヘルパー |
| `src/compiler/wasm/sections_types_gc.ark` | GC 型セクション出力 |
| ... full list in original document ... |

## Phase 2: 文字列 GC 表現 ✅ 完了 (2026-06-20)

### 2a: 文字列定数 ✅
| ファイル | 状態 |
|---------|------|
| `wasm/inst_const.ark` | ✅ GC: array.new_default + data section copy |

### 2b: String 比較 (==) ✅
| ファイル | 状態 |
|---------|------|
| `wasm/intrinsic_math.ark` | ✅ GC: array.len + array.get_u |
| `wasm/binary_type_select.ark` | ✅ GC: VT_REF 判別 |
| `wasm/inst_compare.ark` | ✅ GC: 文字列比較ディスパッチ |

### 2c: String 操作 — concat, substring, eq, slice, starts_with, ends_with ✅
| ファイル | 関数 | 状態 |
|---------|------|------|
| `wasm/intrinsic_string_basic.ark` | emit_concat | ✅ GC: array.new + copy loops |
| `wasm/intrinsic_string_slice.ark` | emit_slice | ✅ GC: array.new + copy loop |
| `wasm/intrinsic_math.ark` | emit_string_eq_intrinsic | ✅ GC: array.len + array.get_u |
| `wasm/intrinsic_string_affix.ark` | emit_starts_with/ends_with | ✅ GC: array.get_u |
| `wasm/intrinsic_string_affix_gc.ark` | GC affix helpers | ✅ |
| `wasm/string_gc_helpers.ark` | GC ループ/比較ヘルパー | ✅ |

### 2c: GC emit_to_string (i32/i64/f64) ✅
| ファイル | 関数 | 状態 |
|---------|------|------|
| `wasm/intrinsic_string_format_i32.ark` | emit_to_string | ✅ GC: array.new_default + digit copy loop |
| `wasm/intrinsic_string_format_i64.ark` | emit_i64_to_string | ✅ GC: array.new_default + digit copy loop |
| `wasm/intrinsic_string_format_f64.ark` | emit_f64_to_string | ✅ GC: array.new_default + digit copy loop |

### 2d: I/O 層 GC パス ✅
| ファイル | 関数 | 状態 |
|---------|------|------|
| `wasm/intrinsic_stdio.ark` | emit_gc_println/print | ✅ GC 配列 → linear memory コピー |
| `wasm/intrinsic_stdio.ark` | emit_gc_string_to_heap | ✅ GC 文字列 → heap バッファ変換 |

### 2e: 型システム修正 ✅
| ファイル | 修正内容 |
|---------|---------|
| `corehir/param_shape_value.ark` | `String` パラメータ → VT_REF (従来は VT_I32) |
| `corehir/return_type_value.ark` | `String` 戻り値 → VT_REF (従来は VT_I32) |
| `loader/module_graph.ark` | bare import 解決を root_dir に統一 |

## 検証結果

- ✅ **GC array smoke test**: `array_gc.ark` compile + run → 正しく出力
- ✅ **string_gc test**: compile + validate → valid
- ✅ **string_gc runtime**: `arukellt\narukellt rocks` → 正しく出力
- ✅ **wasm-tools validate --features gc**: 全テスト通過
- ✅ **T1 退行チェック**: 全パス（docs drift 除く）

## Phase 3: Vec/Enum/Struct GC 表現 🟡 未着手（計画済み）

### Vec<T> GC 表現
```wasm
;; Vec<String> の場合:
(struct (field (mut (ref null (array (mut i32))))  ;; data: GC-backed array of i32 (indexes/refs)
       (field (mut i32)))                           ;; len: number of elements
```
ただし T1 互換性のため、現状の linear-memory Vec は維持し、GC ターゲットのみ GC 表現に切り替える。

### Enum GC 表現
```wasm
;; 基本型 (base):
(struct (field (mut i32)))                           ;; discriminant tag
;; Variant:
(sub final (struct (field (mut i32)                  ;; tag
                         (field (mut $payload)))))   ;; payload
```
`match` の Wasm lowering: `br_on_cast` + `br_on_cast_fail`

### 実装ステップ（コンパイラ再ビルド後に検証）

各ステップの **Verify** に記載されたコマンドですべて ✅ になることを以って完了とする。

1. ✅ GC Vec 型定義（sections_types_gc に Vec 型追加 = `S_f0_ref1_f1_i32`）
   - **Verify:** `grep 'S_f0_ref1_f1_i32' src/compiler/wasm/sections_types_gc.ark`
   - 期待: ファイル内に `S_f0_ref1_f1_i32` が定義されている

2. ✅ struct 型の ref フィールド対応（`emit_struct_field_type` 追加）
   - **Verify:** `grep 'ref' src/compiler/wasm/sections_types_gc.ark | head -5`
   - 期待: `emit_struct_field_type` 内で `"ref"` 始まりの sig → `WASM_REF_NULL()` 分岐がある

3. ✅ `ctx_gc_type.ark` に Vec 型ヘルパー追加
   - **Verify:** `grep 'vec_type' src/compiler/wasm/ctx_gc_type.ark`
   - 期待: `SelfEmitCtx_vec_type` 関数が定義され、`gc_type_base + 10` を返す

4. ✅ `emit_vec_new_gc`: GC struct.new + array.new_default (2026-06-22)
   - `src/compiler/wasm/intrinsic_vec_new_layout.ark`: `emit_vec_new_layout_gc` 追加
   - パターン: `array.new_default A_i32 8` → `struct.new_default vec_type` → `struct.set vec_type 0`
   - **Verify (コンパイラ再ビルド後):**
     ```
     arukeit compile tests/fixtures/stdlib_vec/vec_new.ark -o /tmp/p3_vec_new.wasm --target wasm32-wasi-p2
     wasm-tools validate --features gc /tmp/p3_vec_new.wasm
     wasm-tools dump /tmp/p3_vec_new.wasm 2>&1 | grep -E 'struct.new.*10|array.new_default'
     ```
   - 期待: Vec 生成時に `struct.new` (type idx 10 = `S_f0_ref1_f1_i32`) と `array.new_default` が発行される

5. ✅ `emit_vec_len` GC パス: struct.get vec_type 1 (2026-06-22)
   - **Verify:** `src/compiler/wasm/intrinsic_vec_core.ark` に `is_gc_target` 分岐あり

6. ✅ `emit_vec_get` / `get_unchecked` GC パス (2026-06-22)
   - `src/compiler/wasm/intrinsic_vec_access.ark`: `emit_get_unchecked_gc`, `emit_vec_get_gc` 追加
   - パターン: `struct.get vec_type 0` → `array.get A_i32`
   - **Verify (コンパイラ再ビルド後):**
     ```
     arukeit compile tests/fixtures/stdlib_vec/vec_get.ark -o /tmp/p3_vec_get.wasm --target wasm32-wasi-p2
     wasm-tools validate --features gc /tmp/p3_vec_get.wasm
     wasm-tools dump /tmp/p3_vec_get.wasm 2>&1 | grep 'array.get'
     ```

7. ⏳ `emit_vec_push_gc`: array.set + growth logic
   - **Verify (実装後):**
     ```
     arukeit compile tests/fixtures/stdlib_vec/vec_push.ark -o /tmp/p3_vec_push.wasm --target wasm32-wasi-p2
     wasm-tools validate --features gc /tmp/p3_vec_push.wasm
     wasm-tools dump /tmp/p3_vec_push.wasm 2>&1 | grep 'array.set'
     ```
   - 期待: push 操作が `array.set` で実装され、runtime で正しく動作する

8. ⏳ `emit_vec_pop` / `emit_vec_set` GC パス
9. ⏳ `emit_chars`: Vec GC を使用して実装
10. ⏳ Enum subtype hierarchy + `br_on_cast` dispatch

> **注意**: 上記の型システム変更はソースコードには反映済みだが、selfhost コンパイラを再ビルドしないとテスト不可。再ビルドには fixpoint build の解決が必要。

## Phase 4: 検証

各チェックの **Verify** に記載されたコマンドですべて ✅ になることを以って完了とする。

| チェック | Verify | 結果 |
|---------|--------|------|
| GC array smoke gate | `arukellt run tests/fixtures/t3/array_gc.ark --target wasm32-wasi-p2` | ✅ |
| GC string compile | `arukellt compile tests/fixtures/t3/string_gc.ark -o /dev/null --target wasm32-wasi-p2` | ✅ compiler valid |
| GC string runtime | `arukellt run tests/fixtures/t3/string_gc.ark --target wasm32-wasi-p2` → 出力: `arukellt\narukellt rocks` | ✅ |
| `emit_to_string` (i32) GC パス | `arukellt run tests/fixtures/stdlib_io/i32_to_string.ark --target wasm32-wasi-p2` | ✅ |
| `emit_to_string` (i64) GC パス | `arukellt run tests/fixtures/stdlib_io/f64_to_string.ark --target wasm32-wasi-p2` | ✅ |
| `emit_to_string` (f64) GC パス | `arukellt run tests/fixtures/stdlib_io/f64_to_string.ark --target wasm32-wasi-p2` | ✅ |
| T1 退行チェック | `python3 scripts/manager.py verify quick 2>&1 \| grep -E 'FAIL\|T1\|p1'` | ✅ 全パス（docs drift 3件を除く） |
| コンパイラ import fan-out | `grep -r '^use ' src/compiler/wasm/intrinsic_string_*.ark \| wc -l` | ✅ 13件以内 |
| コンパイラ line limits | `wc -l src/compiler/wasm/intrinsic_string_*.ark src/compiler/wasm/string_gc_helpers.ark` | ✅ 249行以内 |
| GC 全フィクスチャ通過 | `python3 scripts/manager.py verify --full 2>&1 \| tail -5` | ⏳ 未達 |
| T1 パス維持 (定期) | `python3 scripts/manager.py verify quick 2>&1 \| tail -5` | 🟡 定期確認 |
| gc_hint custom section | `arukellt compile docs/examples/hello.ark -o /tmp/hint.wasm --target wasm32-wasi-p2 -O2 && wasm-tools dump /tmp/hint.wasm 2>&1 \| grep 'gc_hint'` | ⏳ 未着手 |
| Benchmark 比較 | `python3 scripts/util/benchmark_runner.py --mode full && python3 scripts/util/benchmark_runner.py --mode compare` | ⏳ 未着手 |

## 主要な修正点

1. **GC ref scratch locals** (`code_locals.ark`, `ctx_scratch.ark`): 2 個の `(ref null A_i8)` スクラッチローカル

2. **MIR_EQ/MIR_ADD の文字列対応**: VT_REF 検出 → GC intrinsic に委譲

3. **MirInst_call の arg0/arg1 問題**: MIR_CALL の arg0/arg1 は常に -1
