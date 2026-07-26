# #807 — Fixture parity: 367 remaining failures クローズ計画

ステータス: 進行中（L3 tranche）  
親 issue: [#807](../../issues/open/807-fixture-parity-367-remaining-failures.md)  
担当 subagent lane: `wave/807-fixture-parity`  
作業 worktree: `.worktrees/wave-807-fixture-parity`  
作成日: 2026-07-25  
更新: 2026-07-26

## 1. 現状とゴール

- レシート起点: `fixture_parity` fail=1089（issue 文面の 367 は古い）。
- 目標: 失敗数をゼロにし、`verify full receipt` を更新する。
- New-failure ratchet: 失敗数は減少のみ許容。

### L2 tranche（2026-07-26）

1. **Harness:** `--filter-dir` 追加。P2 stdio imports を host-linker 経由に（instantiate 失敗の大量解消）。
2. **ネスト lock:** `_ensure_current_selfhost` が flock を再取得しないよう修正。
3. **bool 型名:** `&&`/`||`/`!` の MIR local に `bool` を付与 → `.to_string()` が true/false。
4. **整数 widen:** 共有 `runtime.i8_to_i32` handler が callee 名を見て extend8/16 / zero-extend。

計測:
- コア4ディレクトリ: PASS=41 FAIL=0 SKIP=1
- 全 suite: **PASS=1029 FAIL=327 SKIP=259**（開始レシート fail=1089 から減少）

### L3 tranche（2026-07-26）— structs / tuple store materialization

1. **GC ref STRUCT_GET store:** ref field gets を dest に必ず materialize（`field_access`）。
2. **Chained field assign base:** `lower_expr` が返した local を使い、誤った stack-save を避ける。
3. **Store policy:** `CONST_I32; LOCAL_GET; STRUCT_GET` で RHS を SKIP しない（`count = 42`）。
4. **Tuple destructure:** `STRUCT_GET` の直後に stack `LOCAL_SET` で binding を確定（`variables/tuple`, `generics/two_params`）。
5. **回帰回避:** `STRUCT_GET` を無条件 force-SET しない（`a.x == b.x` stack 合成 / `struct_eq`）。

計測:
- 対象フィルタ: PASS=162 FAIL=14 SKIP=13（L2 後フィルタ FAIL=17 から減少）
- 全 suite: **PASS=1032 FAIL=324 SKIP=259**（L2 の FAIL=327 から -3）
- 残り例: `push_chained_field`, enums 第2 variant trap, `question_mark/*`

### L4 tranche（2026-07-26）— enum bind / chained vec len / GC parse_i32

計測: 全 suite **PASS=1057 FAIL=299 SKIP=259**（L3 の FAIL=324 から -25）。tip `c75ab1f3`。

### L5 tranche（2026-07-26）— `?` From on wasm32-gc + array stack compose

1. **#840 GC From on `?` Err:** extract Err payload → `From::from` → rebuild `Result::Err`（`from_error`）。
2. **ARRAY_* store policy:** `ARRAY_GET`/`SET` を STRUCT 向け local-load 判定から外し、CONST index を stack に残す（`array_literal` / `array_repeat`）。
3. **残:** `from_trait_not_inherent` — From 無し・異種 E の GC identity は typed Err cast で trap（linear では handle 同一性で通る）。#807 継続。

計測: 全 suite **PASS=1062 FAIL=294 SKIP=259**（wasm-invalid=242）。L4 の FAIL=299 から -5。

### L6 tranche（2026-07-26）— concat fallback + MapIter funcref field call_ref

1. **`__intrinsic_concat` fallback:** unresolved concat → `intrinsic_string_basic::emit_concat`（`builder_*` / Debug writers）。
2. **GC struct fn fields:** scalar suffix `_fnref`（was `_i32`）+ field-access dest `VT_FUNCREF`。
3. **`(self.f)(x)`:** field-access callee → `call_ref`；funcref STRUCT_GET を dest に materialize；nullable field → `ref.as_non_null`。
4. **残クラス:** `stdlib_json` parse_value_at trap（62）；binary fold funcref typed as unary（iterator_fold_* invalid）；`from_trait_not_inherent`。

計測: 全 suite **PASS=1094 FAIL=262 SKIP=259**（wasm-invalid=229）。L5 の FAIL=294 から -32。  
ディレクトリ: text 24→16、trait 55→33、json 62→62。

### L7 tranche（2026-07-26）— GC parse_f64 + binary fold fnref2

1. **`parse_f64` GC:** stub `ref.null` Result をやめ、String 走査の実実装（`intrinsic_parse_f64_gc`）。json `parse_value_at` の number trap を解消。
2. **`fnref2`:** binary `fn(i32,i32)->i32` を CoreHIR type_name / MIR param / Wasm type / `ref.func` dest で保持。
3. **call_ref:** `mir_is_funcref_local` と param bind が `fnref2` を VT_FUNCREF として認識（direct CALL→unreachable を回避）。

計測: 全 suite **PASS=1153 FAIL=203 SKIP=259**（wasm-invalid=223）。L6 の FAIL=262 から -59。  
ディレクトリ: json 62→11、trait 33→27、string 20→18。残り top: trait 27、io 21、core 19、string 18、text 16。

## 2. 前提・依存

- #287（fixture parity harness）done。
- `docs/data/verify-full-receipt.json` に失敗リストの正本あり。

## 3. フェーズと完了条件

### Phase 0 — 失敗リスト取得と分類
- `jq '.checks[] | select(.check_id == "fixture_parity") | .items[] | select(.result == "fail")'` で取得。
- 失敗タイプ別に分類:
  - `current wasm trap at runtime, pinned OK`
  - `current wasm invalid, pinned OK`
  - stdout 不一致など

### Phase 1 — ハーネス拡張（必要なら）
- `scripts/selfhost/checks.py` の `_load_manifest_fixtures` に `--filter-dir` オプションを追加し、特定ディレクトリだけの実行を可能にする。

### Phase 2 — 並列サブレーンでバグ修正
- 失敗 fixture をディレクトリ別にサブレーンに分割:
  - arrays, associated_fn, closure_capture
  - control, operators, match_extensions
  - functions, generics, generics_v1
  - enums, for_loops, from_trait, display_trait
  - collections, hashmap, option, result
  - stdlib_core, stdlib_hashmap, stdlib_hashset
  - stdlib_bytes, stdlib_csv, stdlib_env, stdlib_fs
  - stdlib_cli, stdlib_collections_compiler, stdlib_component
  - scalar, operators, question_mark, opt
  - selfhost, integration, host, examples, hello
- 各サブレーンは `wave/807-fixture-parity-<dir>` ブランチを使う。

### Phase 3 — 集約とレシート更新
- 全サブレーンを `wave/807-fixture-parity` に統合。
- `python3 scripts/manager.py verify full` で `verify-full-receipt.json` を再生成。

## 4. 作業レーン・並列可否

- 並列可能。ただし `selfhost fixture-parity` は `runtime_lock` で直列化される。
- 各サブレーンは異なる fixture ディレクトリを担当し、競合を避ける。

## 5. 検証コマンド

```bash
python3 scripts/manager.py selfhost fixture-parity
python3 scripts/manager.py verify full
python3 scripts/manager.py verify quick
```

## 6. リスク

- 新規失敗の追加は回帰とみなされる。
- 失敗原因が自己ホストコンパイラの深い lowering バグの場合、修正が大きくなる。
- `verify full` は時間がかかる。

## 7. 進捗更新規則

- 各ディレクトリ完了後に失敗数を記録し、親オーケストレータへ報告。
- `docs/data/verify-full-receipt.json` は最終統合時に一度だけ再生成する。