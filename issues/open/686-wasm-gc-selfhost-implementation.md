# Wasm GC Selfhost Implementation

- Track: `gc-native`, `compiler`
- Status: **open**
- Depends on: ADR-035

## Summary

Implement Wasm GC (`struct.new`, `array.new`, `ref.cast`, `br_on_cast`) in the
selfhost compiler. ADR-002 chose GC-native in principle (2026-03-25), and the
Rust prototype proved feasibility (542 tests). The selfhost emitter instead
uses linear memory for all targets. This issue tracks the GC rollout per
ADR-035's phased plan.

## 検証方針

各チェックボックスの **Verify** に記載されたコマンドですべて ✅ になることを
以って、その要求を満たしたと証明する。

## Sub-issues / Phases

### Phase 1: Value Representation GC-化 (`035-gc-value-representation.md`) ✅ 基本完了

- [x] MIR type system に GC reference type を追加 (`value_types.ark`, `MirLocal`)
  - **Verify:** `grep -rn VT_GC_REF src/compiler/corehir/value_types.ark src/compiler/mir/`
  - 期待: `VT_GC_REF` が variant として定義され、MirLocal で使用されている

- [x] sig_to_wasm_type で GC type を出力 (reference type encoding)
  - **Verify:** `grep -n 'ref\|gc\|GC_REF\|ref_null' src/compiler/wasm/sections_type_plan.ark`
  - 期待: `"ref"` sig → `WASM_REF_NULL()` または reference type encoding にマッピングされている

- [x] struct.new/struct.get/struct.set の Wasm GC 命令出力
  - **Verify:** 
    ```
    arukeit compile tests/fixtures/structs/basic_struct.ark -o /tmp/struct_gc.wasm --target wasm32-wasi-p2
    wasm-tools validate --features gc /tmp/struct_gc.wasm
    wasm-tools dump /tmp/struct_gc.wasm 2>&1 | grep -E 'struct.new|struct.get|struct.set'
    ```
  - 期待: `validate` が **OK**、dump に `struct.new` / `struct.get` / `struct.set` が含まれる

- [x] array.new/array.get/array.set の Wasm GC 命令出力
  - **Verify:**
    ```
    arukeit compile tests/fixtures/arrays/array_literal.ark -o /tmp/array_gc.wasm --target wasm32-wasi-p2
    wasm-tools validate --features gc /tmp/array_gc.wasm
    wasm-tools dump /tmp/array_gc.wasm 2>&1 | grep -E 'array.new|array.get|array.set'
    ```
  - 期待: `validate` が **OK**、dump に `array.new` / `array.get` / `array.set` が含まれる

- [x] 関数シグニチャの GC reference type 対応 (Wasm バリデーション通過)
  - **Verify:**
    ```
    arukeit compile tests/fixtures/structs/struct_eq.ark -o /tmp/struct_eq_gc.wasm --target wasm32-wasi-p2
    wasm-tools validate --features gc /tmp/struct_eq_gc.wasm
    wasm-tools dump /tmp/struct_eq_gc.wasm 2>&1 | grep 'ref.null'
    ```
  - 期待: `validate` が **OK**、関数シグニチャが `(ref null ...)` でエンコードされている

> **残 precision work:** 固定 S8 struct shape → 本物の shape registry 置き換え。
> f64/i64 field shapes 対応、method/component aggregate ABI coverage は未完了。
> 詳細は `docs/gc-implementation-plan.md` 参照。

---

### Phase 2: 文字列 GC 表現 (`035-gc-strings.md`) ✅ 完了 (2026-06-20)

- [x] String の GC 表現: `(ref null (array (mut i8)))`
  - **Verify:**
    ```
    arukeit compile tests/fixtures/t3/string_gc.ark -o /tmp/string_gc.wasm --target wasm32-wasi-p2
    wasm-tools validate --features gc /tmp/string_gc.wasm
    wasm-tools dump /tmp/string_gc.wasm 2>&1 | grep 'array.new_default.*0'
    ```
  - 期待: `validate` が **OK**、`array.new_default` が type index 0 (`A_i8`) で発行されている

- [x] concat/substring/char_at の GC 配列操作への移行
  - **Verify:**
    ```
    arukeit run tests/fixtures/t3/string_gc.ark --target wasm32-wasi-p2
    ```
  - 期待: 標準出力に `arukellt\narukellt rocks` と正しく出力される
  - 補完検証:
    ```
    # concat: string_concat fixture (t3-run で実行)
    python3 scripts/manager.py verify quick 2>&1 | grep -E 'string_concat|string_slice|PASS'
    ```

- [x] linear memory 上の length-prefixed 文字列からの移行
  - **Verify:**
    ```
    arukeit compile tests/fixtures/t3/string_gc.ark -o /tmp/string_gc_noheap.wasm --target wasm32-wasi-p2
    wasm-tools dump /tmp/string_gc_noheap.wasm 2>&1 | grep -c 'i32.load\|i32.store'
    ```
  - 期待: GC 文字列操作パスに `i32.load` / `i32.store` による手動メモリアクセスが含まれない
    （I/O 層の linear memory コピーは除く）

> 実装詳細は `docs/gc-implementation-plan.md` Phase 2 参照。
> 検証済み: len, ==, starts_with, ends_with, concat, to_string (i32/i64/f64), print/println。

---

### Phase 3: Vec/Enum/Struct GC 表現 (`035-gc-vec-enum-struct.md`)

- [x] Vec&lt;T&gt; の GC struct + GC array backing ✅ 全基本操作完了
  - 内部ステップ:
    - [x] GC Vec 型定義 (`S_f0_ref1_f1_i32` in `sections_types_gc.ark`) — 事前完了
    - [x] struct ref フィールド対応 (`emit_struct_field_type`) — 事前完了
    - [x] `ctx_gc_type.ark` Vec 型ヘルパー — 事前完了
    - [x] `emit_vec_new_gc` — GC struct.new + array.new_default ✅ (2026-06-22)
    - [x] `emit_vec_len` GC パス — struct.get vec_type 1 ✅ (2026-06-22)
    - [x] `emit_vec_get` / `get_unchecked` GC パス ✅ (2026-06-22)
    - [x] `emit_vec_push` GC パス — array.set + growth + `array.copy` on resize ✅ (2026-06-24: growth copy 欠落修復)
    - [x] `emit_vec_pop` GC パス ✅ (2026-06-22)
    - [x] `emit_vec_set` GC パス ✅ (2026-06-22)
    - [x] `emit_chars` Vec GC 使用 ✅ (2026-06-22)
    - [x] `emit_is_empty` GC パス ✅ (2026-06-22)
  - **Verify (全ステップ完了後):**
    ```
    # Vec 型が GC struct として出力されること
    arukeit compile tests/fixtures/stdlib_vec/vec_new.ark -o /tmp/vec_gc.wasm --target wasm32-wasi-p2
    wasm-tools validate --features gc /tmp/vec_gc.wasm
    wasm-tools dump /tmp/vec_gc.wasm 2>&1 | grep -E 'struct.new.*10|struct.get.*10|struct.set.*10'
    wasm-tools dump /tmp/vec_gc.wasm 2>&1 | grep -E 'array.new|array.get|array.set'
    # Vec 操作の runtime 正しさ
    arukeit run tests/fixtures/stdlib_vec/vec_push.ark --target wasm32-wasi-p2
    arukeit run tests/fixtures/stdlib_vec/vec_get.ark --target wasm32-wasi-p2
    arukeit run tests/fixtures/stdlib_vec/vec_len.ark --target wasm32-wasi-p2
    ```
  - 期待: Vec 型が `S_f0_ref1_f1_i32` (type index 10) の struct として出力され、
    runtime テストが正しく動作する

- [x] Enum subtype hierarchy + tag-based match dispatch
  - 内部進捗:
    - [x] `MIR_BR_ON_CAST` / `MIR_BR_ON_CAST_FAIL` オペコード追加 (82, 83)
    - [x] 命令コンストラクタ (`mir/inst_br_on_cast.ark`)
    - [x] wasm backend 発行 (`inst_control.ark`, `inst_dispatch_control.ark`)
    - [x] LowerCtx に `is_gc_target` / `gc_type_base` フィールド追加
    - [x] GC enum type section（open base `Sub0_S_f0_i32` + final variant subtypes）
    - [x] CoreHIR/AST match lowering で variant タグ比較（`gc.struct.get` field 0 + `i32.eq`）
    - [x] Enum variant 構築を VT_GC_REF 化
    - [x] `exhaustive_match.ark` が `wasm-tools validate --features gc` + host-run で `stop/caution/go` を出力
  - **注記 (2026-06-24):** match dispatch は `br_on_cast` ではなく GC struct タグ比較を採用（wasm validate / 実行の安定性優先）。`br_on_cast` インフラは将来の payload cast 用に残置。
  - **注記 (2026-06-24, landing):** variant slot → canonical wasm type index の lookup テーブル (`gc_enum_variant_type_map`) を導入。`decl_layout_payload_field_type_name` が `i32` 等を空文字にしていたバグを修正（`B(i32)` が `ref14` payload になる問題）。`tests/fixtures/enums/*.ark` 全件が `wasm-tools validate --features gc` + host-run 通過。
  - **Verify:**
    ```
    ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2.wasm \
      scripts/run/arukellt-selfhost.sh compile --target wasm32-wasi-p2 \
      tests/fixtures/enums/exhaustive_match.ark -o /tmp/enum_gc.wasm
    wasm-tools validate --features gc /tmp/enum_gc.wasm
    tools/host-linker/target/release/arukellt-host-run /tmp/enum_gc.wasm
    ```
  - 期待: validate OK、`stop` / `caution` / `go` が順に出力される

- [x] HashMap GC 表現
  - **注記 (2026-06-24):** `hashmap_basic.ark` validate + host-run 通過（`200` / `found` / `not found` 出力）。根因は GC `vec_push` growth が `array.new_default` 後に旧要素をコピーしておらず、`hashmap_new` の 9 回目 push で `capacity`（index 0）が 0 に戻り `hashmap_set` が `i32.rem_s` trap していたこと。`intrinsic_vec_push_gc.ark` に `array.copy`（opcode `0xfb11`）を追加して修復。`match_println_i32.ark` host-run も通過。`vec_push.ark` validate + host-run 継続 OK。
  - **注記 (2026-06-25):** `hashmap_string_i32.ark` validate + host-run 通過（期待出力 `3` / `3` / `42` / `not found` / `true` / `false`）。修復: 3 引数 call staging で `local.set_from` を使う（GC 文字列リテラルがローカルに入った後に空スタック `local.set` していた）、GC `bool_to_string` が linear offset のみ残していたのを GC 配列へ materialize。
  - **注記 (2026-06-25, len):** `hashmap_basic.ark` の `HashMap_i32_i32_len` が `0` を出力していた問題を修復。CoreHIR `println` 変換が `CALL` 結果を `LOCAL_GET` で読む際、`CALL`→`LOCAL_GET`→`CALL`（`i32_to_string`）パターンで `local.set` がスキップされ未初期化ローカルを参照していた。`inst_store_policy.ark` で `MIR_CALL`/`MIR_WIT_CALL` に限定して store を強制、`call_fallback.ark` で `LOCAL_SET_FROM` 経路も補完。
  - **注記 (2026-07-01):** `hashmap_i32_string.ark` / `hashmap_string_string.ark` / `hashset_string_basic.ark` の GC validate 失敗を修復。`Option<String>` の `Some` 構築・match bind が i32 payload 形状を選んでいたため、payload VT で `Option::Some` GC variant slot を選択するよう変更。`HashSet<String>` facade の `let _updated: Vec<String> = push(...)` は GC push の void path と衝突していたため statement 呼び出しへ変更。`__hm_is_get_val` / `__hm_ss_get_key` / `__hm_ss_get_val` の String callee 推論も補完。
  - **Verify (実装後):**
    ```
    arukeit compile tests/fixtures/stdlib_hashmap/hashmap_basic.ark -o /tmp/hm_gc.wasm --target wasm32-wasi-p2
    wasm-tools validate --features gc /tmp/hm_gc.wasm
    arukeit run tests/fixtures/stdlib_hashmap/hashmap_basic.ark --target wasm32-wasi-p2
    arukeit run tests/fixtures/stdlib_hashmap/hashmap_string_i32.ark --target wasm32-wasi-p2
    ```
  - 期待: HashMap が GC struct と GC array で実装され、runtime で正しく動作する

- [x] i31ref boxing for small integers in generics
  - **注記 (2026-06-24):** `boxing_i31.ark` validate 通過。`vec_push.ark` validate + host-run 通過（`i32_to_string` 結果ローカル型修復）。
  - **注記 (2026-06-25):** `tests/fixtures/t3/boxing_i31.ark` validate + host-run 通過（出力 `42`）、`wasm-tools dump` で `ref.i31` 確認。
  - **Verify (実装後):**
    ```
    wasm-tools dump /tmp/boxing_test.wasm 2>&1 | grep -E 'i31.new|ref.i31'
    ```
  - 期待: 小整数 (i32) が generic コンテナ内で `i31.new` により boxing される

---

### Phase 4: 検証・最適化 (`035-gc-verification.md`)

- [ ] `--target wasm32-wasi-p2` で全フィクスチャ通過
  - **Verify:**
    ```
    python3 scripts/manager.py verify --full 2>&1 | tail -20
    ```
  - 期待: **0 failed**、全 `t3-run:` / `t3-compile:` / `run:` フィクスチャが通過
  - 内訳確認:
    ```
    grep -c 't3-compile:' tests/fixtures/manifest.txt   # compile-only
    grep -c 't3-run:' tests/fixtures/manifest.txt       # run テスト
    ```

- [ ] T1 linear memory パス維持確認
  - **Verify:**
    ```
    python3 scripts/manager.py verify quick 2>&1 | grep -E 'T1|t1|wasm32-wasi-p1|FAIL'
    ```
  - 期待: T1 (`wasm32-wasi-p1`) の全テストが GC 変更前と変わらず通過
  - 補完: `grep -c 'run:' tests/fixtures/manifest.txt` の全件が PASS

- [ ] gc_hint custom section 充実
  - **Verify:**
    ```
    arukeit compile docs/examples/hello.ark -o /tmp/hello_hint.wasm --target wasm32-wasi-p2 -O2
    wasm-tools dump /tmp/hello_hint.wasm 2>&1 | grep -A5 'gc_hint'
    ```
  - 期待: 出力 wasm に `gc_hint` custom section が含まれ、適切な GC type layout metadata を保持している

- [ ] Benchmark 比較 (T1 linear vs T3 GC)
  - **Verify:**
    ```
    python3 scripts/util/benchmark_runner.py --mode full
    python3 scripts/util/benchmark_runner.py --mode compare
    ```
  - 期待: T1 (linear memory) と T3 (GC) の benchmark 結果が記録・比較可能であること
  - 補完: `docs/process/benchmark-results.md` に T3 GC の計測値が追記されている

## 関連

- ADR-035: Wasm GC Implementation Plan
- Done: #005-#025 (Rust prototype GC issues)
- Depends on: #036/#037 (jco GC support, external)
