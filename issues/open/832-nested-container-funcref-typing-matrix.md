---
Status: open
Created: 2026-07-25
Updated: 2026-07-25
ID: 832
Track: compiler-internal
Depends on: 831
Related: ADR-033, ADR-035, #722, #726, #831
Orchestration class: implementation-ready
Orchestration upstream: 831
Blocks v{N}: none
Priority: 2
Source: post-#831 nested Option/Result/Vec/fn value typing gaps
---

# Nested container / funcref 型付け行列の穴埋め

## Summary

#831 で Class A/B の typed funcref（`ref.func` / `call_ref`）と
`Option<fn>` パラメータ経路は入った。しかし **入れ子コンテナ・関数値の組み合わせ**
では、型名正規化・配列 layout・call-expr callee が揃っておらず、
validate-fail / unreachable / runtime trap が残る。

本 issue は「単発バグ」ではなく、**値として運ぶ型（fn / Vec / Option / Result）の
入れ子行列**を閉じる実装トラックとする。

## Baseline（2026-07-25, 現行 s2 再測定）

旧 `.build/*.wat` の症状表は **stale** を含む。再コンパイル後:

| 形 | 結果 | 備考 |
|----|------|------|
| `let f = double; f(5)` | ✅ | #831 済み（旧 WAT の i32.const は stale） |
| `let f: fn(...) = double; f(5)` | ✅ | |
| `Option<fn>` local `Some(g) => g(5)` | ✅ | パラメータ経路は #831 fixture 済み |
| `Result<fn, String>` `Ok(g) => g(5)` | ✅ | |
| `Option<Vec<i32>>` + `get_unchecked` | ✅ | `b64548d0` で Option payload 正規化 |
| `Result<Vec<i32>, String>` + `get_unchecked` | ✅ | |
| `Option<Option<i32>>` / `Option<Result>` / `Result<Option>` | ✅ | 浅い入れ子は概ね可 |
| `get(v,0) + get(v,1)`（自由 `get`） | ❌ validate | `get` → `Option<T>` を `i32.add` |
| `Vec<fn>` push / get_unchecked / call | ✅ | funcref 配列 ABI（`A_fnref` + vec header） |
| `get_unchecked(fs,0)(5)`（call-expr callee） | ✅ | C: callee 式 lower → `call_ref` |
| `id(double)(5)`（戻り値 fn の即時呼び出し） | ✅ | C: fn 戻り ABI + callee 式 |
| `Vec<Option<i32>>` / `Vec<Result<...>>` | ❌ validate | ref-vs-ref layout |
| `Option<Vec<fn>>` / `Result<Vec<fn>, _>` | ❌ validate | `Vec<fn>` に依存 |
| `Vec<Vec<i32>>` / `Vec<String>`（一部経路） | ❌ runtime | validate は通るが trap |

## Root causes（クラスタ）

### A. Option/Result payload 型名正規化（部分完了）

- **症状**: `Option_Vec` → bind が bare `Vec` → `get`/`push` が unreachable
- **原因**: Result Ok 抽出は `normalize_payload_elem_type_name` を呼ぶが、
  Option 抽出は substring のみだった
- **修正済み**: `b64548d0`（`core_match_payload_bind_core` /
  `match_payload_fields` + fixture `option_match_vec_i32_get.ark`）
- **残り**: 同様の正規化漏れが他 prefix（`Option:`, 深い `option:option:…`,
  local ann shaping の `Option_` vs `option:` 不統一）にないか監査

### B. `Vec<fn>` / funcref 要素配列 ABI（完了）

- **修正済み**: `A_fnref` + `SubF_GS_f0_ref26_f1_i32`、structref 判定から fnref 除外、
  scratch / push / get / new / `call_type_vec` VT、fixture `vec_fn_push_call.ark`
- **注意**: `push_gc_array_scratch` の arity 変更は全呼び出し側を同時更新すること
  （`emit_raw_array_grow_gc` 漏れで s2 invalid → stale s3 が拾われる）

### C. Call-expression-as-callee（完了）

- **修正済み**: fn 戻り ABI（`VT_FUNCREF`）+ callee 式 lower → `call_ref`、
  CALL 結果の FUNCREF store 強制。fixtures: `fn_return_bind` / `call_expr_callee`

### D. `Vec<Option<T>>` / `Vec<Result<T,E>>` 要素 layout

- **症状**: validate `expected (ref null $type), found (ref null $type)`
  （型 index 不一致）
- **原因**: 要素が enum-open / 具象 variant の取り違え、または
  `gc_vec_elem_name_is_enum_like` 後の fallback（i32 配列）と
  実際の `Some`/`Ok` 値（GC ref）の不一致
- **必要**: 要素型に応じた enum-ref 配列（または既存 open-ref 配列）を
  一貫して選び、push/get の cast を揃える

### E. 自由関数 `get` → `Option<T>` と算術の組み合わせ

- **症状**: `get(xs,0) + get(xs,1)` が `i32.add` に Option ref を渡して validate-fail
- **原因**: `vec_get` は仕様どおり `Option` を返す。旧 probe の「Result は ✅」は
  WAT に `array.get` が見えただけで、実行/validate は通っていない
- **扱い**:
  - 言語仕様として `get_unchecked` / `match get(...)` を正規とするなら
    **診断**（Option を算術に使ったら型エラー）を強化
  - あるいは糖衣で自動 unwrap は **しない**（ADR-048 / 明示性）
  - fixture・docs を `get_unchecked` または match に寄せる

### F. Nested `Vec<Vec<_>>` / `Vec<String>` runtime trap

- **症状**: validate OK だが `_start` で trap
- **原因候補**: structref-vec layout と `Vec_new_i32` 初期化の不一致、
  get_unchecked の array type 取り違え（#726 系の残骸）
- **必要**: 最小 fixture で array type index / cast を突き、#726 残件と重複整理

## Workstreams（推奨順）

1. **C — call-expr callee → call_ref**（小さく、HOF 合成の基盤）
2. **B — `Vec<fn>` funcref 配列 ABI**
3. **D — `Vec<Option/Result>` 要素 layout**
4. **A 監査 — Option/Result 型名の残漏れ + local ann shaping 統一**
5. **E — `get` 誤用の診断 / docs / fixture 寄せ**
6. **F — nested Vec runtime trap**（#726 と重複ならそちらへ merge）

並列可: C ∥ A 監査。B 完了後に Option/Result&lt;Vec&lt;fn&gt;&gt; を閉じる。

## Primary paths

- `src/compiler/mir/lower/core_call_direct_args.ark` / `call_indirect_emit.ark`
- `src/compiler/mir/lower/core_match_payload_*.ark` / `match_payload_*.ark`
- `src/compiler/mir/lower/call_type_vec.ark` / `call_rewrite_vec*.ark`
- `src/compiler/corehir/type_ann_local_name*.ark` / `type_ann_param_name.ark`
- `src/compiler/wasm/sections_types_gc_phase7.ark` / `intrinsic_vec_type*.ark`
- `src/compiler/wasm/intrinsic_vec_push*.ark` / get 系
- fixtures（新設想定）:
  - `tests/fixtures/functions/fn_var_infer.ark`
  - `tests/fixtures/functions/vec_fn_call.ark`
  - `tests/fixtures/functions/call_expr_callee.ark`
  - `tests/fixtures/collections/vec_option_i32.ark`
  - `tests/fixtures/stdlib_option_result/option_match_vec_i32_get.ark`（既存）

## Non-goals

- `Option` の `None = null` / `br_on_null`（ADR-035）
- Class C `call_indirect` 全廃
- HashMap&lt;K, fn&gt; 完全対応（B の後続で別 issue 可）
- `get` の暗黙 unwrap 糖衣

## Progress

- [x] **C** — call-expr callee / fn 戻り（`c556263b`）
- [x] **B** — `Vec<fn>` funcref 配列 ABI + `vec_fn_push_call`
- [ ] **D** — `Vec<Option/Result>` 要素 layout
- [ ] **A 監査** / **E** / **F**

## Acceptance

- [ ] 上表の ❌ 行がすべて validate + hosted run で期待出力
- [x] `get_unchecked(fs, 0)(5)` と `id(double)(5)` が `call_ref`
- [x] `Vec<fn(i32)->i32>` の push/get_unchecked/call が funcref 配列で通る
- [ ] `Vec<Option<i32>>` / `Vec<Result<i32,String>>` が validate + run
- [ ] 自由 `get(...)+get(...)` は型エラー（または明示ドキュメント + lint）で失敗し、
      silent validate-fail にならない
- [x] 回帰 fixture を `run:` + `t3-compile:` に登録（C/B 分）
- [x] `python3 scripts/manager.py verify lane --gate t3`（B 完了時）
- [ ] フェーズ完了時 `python3 scripts/manager.py verify quick`

## Notes

- #831 acceptance の `Option<fn>` は **パラメータ**経路。local
  `let f: Option<fn> = Some(double)` は現行 s2 では動くが、入れ子行列は未カバーだった
- probe 用ソースは `.build/probe_*.ark`（揮発）。恒久化するなら fixtures へ移す
- `Result_Vec` が動いて `Option_Vec` だけ壊れていた非対称は A で説明済み

## Related

- #831 call_ref emitter（upstream、Class A/B + Option&lt;fn&gt; param）
- #722 typed funcref 計測
- #726 GC ref 型推論 / validate-fail（nested enum・Vec layout と重複しうる）
- ADR-033, ADR-035, ADR-043
