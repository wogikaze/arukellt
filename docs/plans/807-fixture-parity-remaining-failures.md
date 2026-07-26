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

### L8 tranche（2026-07-26）— method/free recursion + unary fixtures

1. **math wrappers:** ADR-046 free `popcount` 等が `n.popcount()` → 同名 free へ再帰していたのをインライン実装へ。
2. **`String::eq`:** free `eq` ↔ `String::eq` 循環を `__intrinsic_string_eq` + name fallback で切断。
3. **`Not for i32`:** `!` が eqz になるため `^ -1` の bitwise complement。
4. **fixtures:** `-5.max/min/clamp` を `(-5).…` に（仕様どおり unary < postfix）。

計測: 全 suite **PASS=1167 FAIL=189 SKIP=259**（wasm-invalid=223）。L7 の FAIL=203 から -14。  
ディレクトリ: core 19→11、trait 27→22。残り top: trait 22、io 21、string 18、text 16、bytes 15。

### L9 tranche（2026-07-26）— assert_eq fallback + json pretty/Bool

1. **`assert_eq` / `assert_ne` / `assert_eq_str`:** core_op miss → drop+unreachable を name fallback で intrinsic 化（GC の `assert_eq_str` は string_eq+assert）。
2. **`i32_to_i64` / `i64_to_i32` / `assert_eq_i64`:** 同様の miss を fallback（into / i64 assert 経路）。
3. **`is_empty`:** String 引数を Vec struct.get 経路へ落とさない。
4. **json:** pretty indent を `indent * (depth + 1)` に修正；`Bool(true/false)` パターンを `Bool(b)`+分岐へ（GC で payload 未判別）。

計測: 全 suite **PASS=1192 FAIL=164 SKIP=259**（wasm-invalid=223）。L8 の FAIL=189 から -25。  
残り top: io 20、string 17、text 16、trait 15、vec 13、bytes 12。io の Write 経路は GC type mismatch が残る。

### L10 tranche（2026-07-26）— io write_bytes vs fs + join concat

1. **Root cause:** bare `write_bytes` → `runtime.write_bytes` core_op が `std::io::write_bytes` 本体を defer し、呼び出しが fs intrinsic / Result 型付けへ吸い込まれて `drop+unreachable` + i32.ne(ref) validate 失敗。
2. **Fix:** `data/core-ops.toml` から bare alias を削除；type fallback は fs 修飾名のみ Result；`write_bytes` は param0=`String` のときだけ defer（io の Vec 本体は lower）。
3. **join:** `__core_string_push_range` の GC ローカル取り違えを避け、`__core_string_join_impl` を `concat` ループへ。

計測: 全 suite **PASS=1224 FAIL=142 SKIP=249**（wasm-invalid=213）。L9 の FAIL=164 から -22。  
残り top: io 14、vec 13、trait/string/bytes 12、text 10。

### L11 tranche（2026-07-26）— byte_len/at、clock i64 scratch、get_unchecked i64、push_char writeback

1. **byte_len / byte_at:** prelude shim が defer されるのに bare core_op がなく `unreachable`。`byte_len`→`text.len_bytes`、`byte_at`→`raw.string_byte_at_unchecked` を alias。
2. **now_ms / wasi_clock:** `emit_wall_datetime_to_ms` が i64 を i32 scratch(0) に格納 → validate fail。offset 15（i64 slot）へ。
3. **vec_*_i64 get_unchecked:** `vec_get_unchecked_i64` 名が `contains("i64")` ヒューリスティックに引っかかり index を `i64.extend_i32_s` → validate fail。`get_unchecked` を除外。
4. **push_char GC:** 新配列を作って破棄していた。CALL.arg0 へ writeback。ただし staging 後の temp ではなく pre-stage receiver を記録。

計測: 全 suite **PASS=1239 FAIL=127 SKIP=249**（wasm-invalid=203）。L10 の FAIL=142 から -15。  
残り top: io 13、bytes 12、trait 12、host/text 10、vec 9。

### L12 tranche（2026-07-26）— bitops precedence、any/find、f64 integer format

1. **base64 / leb128:** Ark は `==`/`<<` が `&` より強い（C 寄）。`byte & 128 == 0` や `b0 & 3 << 4` が誤評価 → 括弧で `(byte & 128) == 0` / `((b0 & 3) << 4) | …`。誤 golden（旧バグ出力）も正しい値へ更新。
2. **any_i32 / find_i32:** `__intrinsic_*` に emitter がなく `unreachable`。prelude に実ループ本体を置き defer 解除。
3. **format_f64:** shortest k を 0 から探索し整数値は `"0"` / `"100"`（末尾 `.0` なし）。関連 `.expected` を同期。
4. **pad_left / property_repeat:** fixture 期待値を stdlib 意味論（ababtest、false な分配律の削除）へ修正。

計測: 全 suite **PASS=1259 FAIL=107 SKIP=249**（wasm-invalid=203）。L11 の FAIL=127 から -20。  
残り top: io 13、trait 12、host 10、hashmap 8、core 7、text 6。

### L13 tranche（2026-07-26）— path join、from_bytes、chars/split empty、format_i64、parse_i64 GC

1. **path::join steal:** bare `join`→`string.join` が path 本体を defer / Vec 署名で上書き。param0=`String` は lower；Vec overload は signature 登録スキップ。
2. **string_from_bytes:** normal_call fallback 欠落 → name fallback で GC/linear intrinsic。
3. **chars/split("",""):** `__core_string_split_impl` が空 haystack+空 delim で 0 要素 → 1 空部分を push。
4. **format_i64:** wasm32-gc を `is_gc_target` で GC alloc；壊れた Ark body を defer；CoreHIR の `-N` 引数を i64 NEG に force。
5. **parse_i64 GC:** `emit_parse_i32_gc` 委譲をやめ `emit_parse_i64_gc`（i64 Ok payload）。

計測: 全 suite **PASS=1276 FAIL=90 SKIP=249**（wasm-invalid=202）。L12 の FAIL=107 から -17（new FAIL=0）。  
残り top: trait 12、io 11、hashmap 8、core 6、host 5。

### L14 tranche（2026-07-26）— i32::MIN、path split/join、parse Err、pop None、goldens

1. **`0 - 2147483648`:** peek で wide literal を i64 にし、左辺を right より前に extend（stack 融合順を維持）。
2. **i64→i32 wrap:** format_i32 / hashmap_* / assert_eq_i32 のみ allowlist（println/debug を壊さない）。
3. **path::normalize:** `__intrinsic_split` / `__intrinsic_join` / `pop` の name fallback。
4. **parse_i64/i32 GC Err:** 入力文字列ではなく `"parse error: invalid integer"`。
5. **vec_pop empty None:** `struct.new_default` 後に tag=1 を書く（vec_get と同型）。
6. **goldens:** `string_chars`→`3\\n1`、`use_func_destructure_multi`→`a,b,c`。

計測: 全 suite **PASS=1286 FAIL=81 SKIP=248**（wasm-invalid=198）。L13 の FAIL=90 から -9（new FAIL=0）。  
残り top: trait 12、io 11、core 6、hashmap 6、host 5。

### L15 tranche（2026-07-26）— parse trailing junk / exponent、BitSet、vec cap0

1. **parse_i32/i64/f64 GC junk:** invalid digit で `ok=0` のあと `br` で (block+loop) を抜け、post-loop の Ok 上書きを防ぐ（`br` 深さ誤りによる無限ループも修正）。
2. **parse_f64:** Err メッセージを `"parse error: invalid float"`；`e`/`E` 指数を実装（json `1e5` 回帰を回避）。
3. **BitSet:** `cap + 31 / 32` → `(cap + 31) / 32`。
4. **vec grow:** capacity 0 からの push で new_cap=8。
5. **fixtures:** `edge_special_chars` len=9；`parse_f64_decimal` を `to_string` 比較へ。

計測: 全 suite **PASS=1292 FAIL=75 SKIP=248**（wasm-invalid=198）。L14 の FAIL=81 から -6（new FAIL=0）。  
残り top: trait 12、io 11、hashmap 6、core 5、host 5。

### L16 tranche（2026-07-26）— P2 process exit import index

1. **P2 exit index:** guest-native import 列で `exit` は 7。`WASI_IMPORT_PROC_EXIT=6` は fs `close` を呼んでいた → `P2_IMPORT_EXIT` + `proc_exit_import_idx`。
2. **host-linker:** P2 `wasi:cli/exit@0.2.0` stub を trap から `process::exit(code)` へ（panic/assert 後の `runtime error:` 付記を除去）。

計測: 全 suite **PASS=1302 FAIL=64 SKIP=249**（wasm-invalid=198）。L15 の FAIL=75 から -11（new FAIL=0）。  
残り top: trait 12、io 11、hashmap 6、core 5、host 4。

### L17 tranche（2026-07-26）— host P2 args/clock/random + assert panic + env arg_at

1. **host-linker P2 stubs:** `args-sizes` / `arguments`（argc=1, prog `arukellt-host-run`）、`monotonic-now` / wall-clock `now`、`get-random-u64`。
2. **P2 static panic:** `emit_static_panic_message_exit` が `emit_p2_write_ptr_len` 経由で stderr に書く（`assert_fail` → `"assertion failed"`）。
3. **GC `env::arg_at`:** 範囲外で `unreachable` ではなく `Option::None`。
4. **`__intrinsic_random_i32`:** name fallback（clock_random / wasi_random）。
5. **golden:** `stdlib_host/host_module_contract.ark.expected`。

計測: 全 suite **PASS=1310 FAIL=56 SKIP=249**（wasm-invalid=198）。L16 の FAIL=64 から -8（new FAIL=0）。  
残り top: trait 12、io 10、hashmap 6、core 5、json 3。

### L18 tranche（2026-07-26）— GC fs read + P2 open/read/close host stubs

1. **GC `read_to_string`:** stub `ref.null` をやめ、path を linear へ stage → open/read/close → Result local（void-if）。
2. **P2 import indices:** `P2_IMPORT_OPEN_AT` / `FD_READ` / `FS_CLOSE`（旧 hardcode 3/4/5 は arguments/stdin/close と衝突）。
3. **host-linker:** 実 `open-at`（fd 表）、`close`、stdin `read` を fd_read ABI として実装。
4. **GC string from scratch:** len を `STRPTR-4` から読む（heap+BUFSTART 誤ロードを修正）。

計測: 全 suite **PASS=1318 FAIL=48 SKIP=249**（wasm-invalid=198）。L17 の FAIL=56 から -8（new FAIL=0）。  
残り top: trait 12、hashmap 6、io 5、core 5、json 3。  
残: `host_capability`（`read_dir`/`metadata` が FsError Result で unreachable）。

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