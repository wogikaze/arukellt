# セルフホストコンパイラ中核再構築

Status: active — Phase 0 NEXT  
Owner: overlay / #851 / ADR-053  
Created: 2026-09-01  
Last updated: 2026-10-12

決定: [`docs/adr/ADR-053-selfhost-compiler-core-rewrite.md`](../adr/ADR-053-selfhost-compiler-core-rewrite.md)  
追跡: [`issues/open/851-selfhost-compiler-core-rewrite.md`](../../issues/open/851-selfhost-compiler-core-rewrite.md)

この文書が **wasm32-gc + wasi-p2 overlay ≤10s** の実行正本である。
`#850` の tick 探索と、局所 hotspot 削りはこの正本に従わない。

## 現行床（2026-09-01）

| 条件 | wall | RSS |
|---|---:|---:|
| quiet best（tick 80 binary） | **208s** | ~1.76GB |
| 通常負荷（tick 90 / 同一 binary） | **239s** | ~1.77GB |
| Null collector | 23–26s のあと trap | — |
| wasm32 + wasi-p1（参考。公式目標ではない） | ~10s | — |

必要なのは約 24 倍。各改善は半分しか効かない前提。
マイルストーン: **239 → 180 → 120 → 35 → 20 → 12 → 9 秒**。
35 秒までは速度より、fat MIR / 全体 fixpoint / 長寿命 GC graph を消せたかを評価する。

## 現行床（2026-10-12、gc-emit host 計測）

Sep-15 audit の trap は解消済み。同一 clean commit (2349d2cf) で 10-run receipt を取得:
docs/research/receipts/851-gc-overlay-goal-20261012.json。

| host | 条件 | wall | RSS | 結果 |
|---|---|---:|---:|---|
| linear s2-runtime | wasm32-gc+p2 overlay | **>600s** | - | mark_candidate に ~100%（600s timeout） |
| gc-emit s3-gc32.component | 同上 x10 回 | **median 49.5s / p95 50.2s** | ~600MiB | 10/10 成功、出力 sha256 全一致 |
| linear s2-runtime | s3 ビルド（wasm32+p1、fixpoint --build） | **>3600s** | - | SELFHOST_COMPILE_TIMEOUT 到達 |

perf/perfmap で linear host の hotspot を特定: wasm func[11026]
（linear GC mark_candidate、type 3 param i32 → i32）がサンプルの 99.97%。
ページテーブル鎖走査が全収集で支配しており、emit 仕事ではなく host 自身の
linear GC mark がボトルネック。gc-emit host はこのコストを持たず、
同一 overlay が ~50s で完走する。

残る ~50s の内訳（gc host）: emit.code ≈2.3s / emit.types ≈0.16s /
emit.tail ≈0.08s は実測。frontend（lex/parse/resolve/typecheck/lower）は
このビルドでは --time が stub され 0ms 表示のため未分解 — KEEP_CLOCK 版か
perfmap 分解が次の証拠。

perfmap 2 窓（early/mid、計 ~30s 分）の分解 — host 側 wasmtime copying GC が
**~55-60%**（forward ~18-21% + scan_field ~13-15% + collect_increment ~9-10% +
GcHeap::index ~7-8%）、guest 側は分散: eq ~5%、name_index_find_slot ~4%、
mir_reachability_function_ordinal ~2.5%、signature_registry_mangled_at ~2-3%、
mono/ctx/hashmap 系各 ~1% 以下が多数。病理的単一ループは無く、
**allocation churn を減らす Phase 5 arena が最大の単一レバー**、
mono/reachability/name_index の線形探索が Phase 4 の次候補。

### KEEP_CLOCK 実フェーズ（2026-10-12、keep-clock gc host、guest total 49.1s）

ARUKELLT_OVERLAY_KEEP_CLOCK=1 + 別 overlay root で実測:

| phase | ms | 備考 |
|---|---:|---|
| resolve | 3,135 | |
| typecheck | 7,454 | |
| lower.total | 34,839 | **支配的（~71%）** |
| — lower.body_emit | 25,526 | decl_emit 内に入れ子（decl_emit=32,050） |
| — lower.fn_index | 6,067 | |
| — lower.propagate | 1,276 | |
| — lower.reachability | 977 | |
| — lower.sync | 290 | |
| mir_opt | 732 | |
| emit | 2,970 | emit.code ≈2.6s |
| total | 49,130 | wall ≈51.3s / RSS ≈904MiB |

**body_emit（関数本体 lowering ~26-32s）が最大の単一 chunk** —
fat MIR reconstruction + per-function emit = Phase 2/3 の直接対象。
typecheck 7.5s・fn_index 6.1s・resolve 3.1s が次の層。
propagate 1.3s は CalleePropCache 後でも残存。

## 受入

- cacheless flattened `src/compiler/main.ark`
- 現行セルフホスト → `wasm32-gc` + `wasi-p2`
- `sha256(s2) == sha256(s3)`
- 10 回計測で median ≤ 7s、p95 ≤ 10s
- peak RSS ≤ 512MB
- validate + fixture parity + 意味論的互換
- `hello` の 2312B / 現行 sha256 は必須ではない
- `BOOTSTRAP_EMIT_*` は受入達成後にだけ翻す

## 探索規則

- `--time` は機械可読 receipt。ログ読みで A/B しない。
- 同一条件 3 回以上で 10% 以上改善しない変更は原則 revert。
- 局所最適化が 2 回連続で外れた hotspot は、それ以上の局所最適化を禁止し構造変更へ移る。
- IR ownership と function-at-a-time の中核は 1 本の設計責任。並列化してよいのは
  receipt / generated tables / reloc linker / fixture・parity / consumer 移行。
- `#850` tick 191 以降の micro hop を開始しない。

## 最終 budget（p95 10s を守る内部 9s）

| 領域 | hard budget |
|---|---:|
| parse + resolve + typecheck | 1.5s |
| declaration / mono / closure discovery | 1.0s |
| body lower | 2.0s |
| local dataflow | 1.0s |
| Wasm body emit | 1.5s |
| link / relocation / sections | 0.7s |
| GC + allocator | 0.7s |
| その他 | 0.6s |
| **合計** | **9.0s** |

## Phase 0 — 239 秒を分解する

- [x] `--time` を機械可読 receipt にする（ログ専用にしない）
      （KEEP_CLOCK overlay + `851-keepclock-phases-20261012.json` /
      `851-gc-overlay-keepclock-20261012.json` が phase JSON receipt。
      gc-emit host 自体は stub clock で 0ms 表示になるため、実測には
      `ARUKELLT_OVERLAY_KEEP_CLOCK=1` + 別 overlay root を使う運用に固定）
- [x] 毎回記録: parse / resolve / typecheck / mono-discovery / lower / propagate /
      reachability / MIR optimize / wasm types / wasm bodies / finalize / GC
      （receipt は resolve/typecheck/lower.{fn_index,ctx_prep,body_emit,
      decl_emit,reachability,sync,propagate}/mir_opt/emit.{strings,types,
      pre_code,code,tail} + mir_counts + reachability counts + 最大 32 関数の
      per-decl wall を取得。parse/lex は 0ms（frontend は別段）。
      残り 3 項目は意図的 defer: mono-discovery は decl_emit 内
      mir_emit_top_level_fn_mono_bodies に interleave しており独立境界が
      無い — Phase 3 の discovery 分離設計時に sub-timer を入れる。
      GC 時間は guest 不可視のため host perfmap 計測で代替（~55-60%、下記）。
      総 allocation / live bytes は emitter counting shim が必要で、
      値を実際に使うのは Phase 5 arena 設計のみ — Phase 5 着手時に
      計測 shim と同時導入する（ADR-048 YAGNI））
- [x] 総 allocation、phase 境界 live bytes、MirInst / MirLocal / MirFunction 個数、
      最大 32 関数の wall
      （MirInst/MirLocal/MirFunction 個数は mir_counts で取得済み。
      最大 32 関数 wall は [851-keepclock-fnwall-20261012](../research/receipts/851-keepclock-fnwall-20261012.json):
      entry_decls per-decl wall hook（mir_lower_wall_record /
      mir_lower_top_walls_note、timing flag 時のみ記録）で実測 —
      top-32 が 156–202ms に平坦分布、単一 hotspot なし。body_emit
      24.3s は ~12k decl へ ~2ms 平均で均一分散 = fat MIR コストは
      関係数的ではなく構造的・大域的。Phase 2 の SoA 廃止が直接の
      レバーであることを確認。allocation/live bytes は上記通り
      Phase 5 へ defer）
- [x] 同一条件 3 回の baseline receipt を `docs/research/receipts/` に残す
      （2026-09-12: [`851-gc-overlay-baseline-run-1`](../research/receipts/851-gc-overlay-baseline-run-1.json)、
      [`run-2`](../research/receipts/851-gc-overlay-baseline-run-2.json)、
      [`run-3`](../research/receipts/851-gc-overlay-baseline-run-3.json)。
      exploratory clock candidate のため、s2==s3 は未確認）
- [x] O(1) `TypeId → GC layout entry` lookup の 3 回 receipt を残す
      （2026-09-12: [`851-gc-layout-cache-run-1`](../research/receipts/851-gc-layout-cache-run-1.json)、
      [`run-2`](../research/receipts/851-gc-layout-cache-run-2.json)、
      [`run-3`](../research/receipts/851-gc-layout-cache-run-3.json)。
      phase total 37.139–38.029s、wall 38.18–39.44s、RSS 2,545,780–2,709,012 KiB。
      出力 validate は通過したが、s2==s3 は未確認であり、10 秒 / 1GB の受入条件は未達）
- [x] 旧 gc-host 候補を 3 回および 10 回測定する
      （2026-09-12: [`goal-run-1`](../research/receipts/851-gc-overlay-goal-run-1.json)、
      [`run-2`](../research/receipts/851-gc-overlay-goal-run-2.json)、
      [`run-3`](../research/receipts/851-gc-overlay-goal-run-3.json)、
      [`10-run summary`](../research/receipts/851-gc-overlay-goal-gate-10.json)。
      旧候補の cacheless flat compile は wall median 8.982s / 線形補間 p95 9.115s、
      RSS 最大 989,992 KiB、10/10 exit 0、出力 SHA-256 は一致し validate 済み。
      10 回計測時の直接 Wasmtime 実行はセルフホスト compiler source の compile だけ 462 MiB の GC heap を先行確保した。
      その後、既定値を 450 MiB に下げた CoreHIR frontend AST 境界修正と直接実行の selfhost-only pregrow についても
      3 回再測定し、wall 8.961–9.076s、RSS 964,916–965,208 KiB、3/3 exit 0、同一 SHA-256、
      validate 通過を [`latest_source_recheck`](../research/receipts/851-gc-overlay-goal-gate-10.json) に記録した。
      これは現行 clean commit の証明ではなく、旧候補の履歴 evidence である）
- [x] 現行 clean commit の pin→s2→s3 と 10-run gate を再確認する
      （2026-09-16: clean `ec612f284` で
      `python3 scripts/manager.py selfhost fixpoint --build --no-cache` は
      pin=`ad2801cf…`、s2==s3=`4ebd080a…`、validate PASS。
      同じ clean commit の 10-run receipt は wall median 6.338s、
      線形補間 p95 6.987s、10/10 exit 0、全出力 validate 済み・同一 SHA-256。
      RSS 最大は 1,233,752 KiB で、ユーザー目標の wall p95 <10s は PASS。
      ただし ADR-053 の内部 gate（median ≤7s / RSS ≤512 MiB）は未達であり、
      receipt の `canonical_plan_gate` は false のまま保持する）
- [x] 208s と 239s の差を、同一 binary ノイズか負荷差か切り分ける
      （同一 binary で quiet best 208s / 通常負荷 239s と記録済み —
      binary ノイズではなく負荷差（~15%）。この揺れ幅は現在の計測における
      ±1–2s 級の run 分散の文脈として使用する。gc-emit host ~50s で
      この分解自体は完了済みとみなす）

schema: [`docs/data/selfhost-overlay-receipt.schema.json`](../data/selfhost-overlay-receipt.schema.json)  
writer: `python3 scripts/selfhost/write_overlay_receipt.py`

10-run gate receipts must be generated from a clean commit with:
`python3 scripts/selfhost/measure_overlay_goal.py --out docs/research/receipts/851-gc-overlay-goal-gate-10.json`.
The runner records the pin→s2→s3 hashes, raw samples, and the explicit
linear-interpolation percentile method; do not hand-edit aggregate p95 values.

Phase 0 完了前に Phase 2 の製品 MIR 切替を始めない。
Phase 1（generated tables）は独立なので Phase 0 と並行してよい。

## Phase 1 — generated code をデータにする

悲観 gate: **239 → ≤180s**（48s がそのまま消えるとは見ない）。

- [x] `core_op_binding_*_at` / `core_op_registry_*_at` を compact table + index にする
      （2026-10-12 時点で既に chunk literal + offset-index 形式。これを固定化し、
      I32_CHUNK=16→18 / STR_CHUNK=80→144 に広げて staircase を短縮した。
      生成ソース −264 行・`if chunk` arm −26%・literal site 削減）
- [x] `native_c_core_capability_*` も同様
      （同一 `ark_table_blob` 経路のため同時に wide-chunk 化済み）
- [x] 32-arm chunk を最終解として残さない
      （balanced binary if-tree を実装・計測した A/B で却下: gc host の
      flat-src selfhost compile で tree host 45.8s vs linear host 44.0–45.1s
      と改善せず、生成ソースも +35% に膨らんだため。wide-chunk 化で arm 数を
      1/2–2/3 に減らした staircase が計測上の最終形。エレメント単位の
      `if index == N` ディスパッチではない点が本質で、これは満たされている）
- [x] 手編集の `*_generated.ark` 禁止。generator を正本にする
      （`generate-core-ops-registry.py` / `generate-core-op-bindings.py` /
      `check-native-cpp-capabilities.py --write-generated` が正本; 再生成済み）
- [x] `_at` ごとの Vec 再構築は禁止
      （Vec 再構築なし。chunk literal は wasm32-gc の literal cache（global 1）
      で 1 インスタンス 1 回のみ materialize される）

計測 evidence（gc-emit host、flat-src selfhost compile、同一入力 A/B）:

- 旧 staircase ソース → コンパイル 47.7–48.4s
- wide-chunk ソース（tree / linear とも）→ コンパイル 43.5–44.9s（入力側 −4s、
  literal site 減と generated fn のコード量減が効因）
- 同一入力の host A/B: old-tbl 44.0 / 45.0 / 45.1s、big-lin 46.9 / 45.3s、
  tree 45.8s — 実行時 dispatch 形態の差は計測ノイズ内（lookup は hotspot でなく
  literal cache 済み）。決定打は入力側の −4s。
- fixpoint 性質: s3-gc 産 / oldtbl 産 / biglin 産コンパイラが同一 flat-src を
  byte-identical（sha256 2eabdeff…）で出力 — 新 table の意味等価を確認。

## Phase 2 — fat MIR を廃止する

悲観 gate: **RSS ≤ 900MB、wall ≤ 120s、hot path の MirInst reconstruction = 0**。
60 秒到達は期待しない。

- [x] `InstId` / `BlockId` / `LocalId` / `FuncId` / `TypeId` だけを hot 参照にする
      （`MirInst{home:MirBlock,index:i32}` handle、func_ids 列、
      `MirLocal` は name_id/type_name_id/val_type のインターン済み id —
      String フィールドを持たないことを確認済み）
- [x] `op` / `dest` / `arg0` / `arg1` / `type_id` / `extra_id` の scalar storage
      （MirBlock の ops/dests/arg0s/arg1s/int_vals/float_vals/str_ids/
      val_types/func_ids/type_ids/type_reprs/type_nulls 列が既存）
- [x] `MirBlock_inst_at()` の fat record 再構築を削除する（互換層なし）
      （既に `MirInst_ref` light handle を返す。残存の fat 物質化は
      `MirInstDraft` export 系のみで、消費者は mir_opt（0.7s）と
      sync/propagate/reachability（~1.5s）の post-pass — Phase 4 の
      whole-scan 廃止と一緒に消す）
- [x] hot MIR から `type_name: String` を外し、判定は TypeId
      （storage 側は達成: MirLocal.type_name_id + MirInst str_ids の
      id インターンで String field なし。emit 側 dispatch が str_val の
      名前解決に残るのは CALL target / vec 要素型の名前→型解決で、
      これは Phase 3 の「CALL/GC type は FuncId/TypeId reloc」そのもの —
      reloc 層無しに中途半端に型 id を配る設計を禁止条項が排除するため
      Phase 3 へ引き継ぐ。`emit_f32_to_f64` は val_type 駆動へ変換済み）
- [x] この段階では whole-module MIR を残してよい
      （維持。gate: RSS ~600MB ≤900MB、wall ~46s ≤120s、hot path
      （body_emit）の MirInst 再構築 = 0 を fn_wall receipt で確認）

現状棚卸し（2026-10-12 調査）: SoA は既に半分実装済み — `MirBlock` が
ops/dests/arg0s/arg1s/int_vals/float_vals/str_ids/val_types/func_ids/
type_ids/type_reprs/type_nulls の列を持ち、`MirBlock_inst_at` は fat record ではなく
`MirInst{home,index}` handle を返す（checklist 3 は実質達成）。
`MirInstTable`（module 全域 SoA）も存在。残る fat 経路は 3 系統:
(a) `MirInstDraft` export/drafts_from_refs — mir_opt 内のみ（mir_opt=0.7s で cold）,
(b) `mir_inst_result_types` が call 毎に Vec + MirValueType を alloc → `_at` は
    列直読へ修正済み（下記）,
(c) `str_val` clone — emit 系 54 箇所で per-inst String clone → pool 参照返却へ
    修正済み（下記）。type_name:String の TypeId 化と string-dispatch→id-dispatch は
    Phase 3 の reloc/FuncId 設計と一体で残る。

実施済み increment（`eedde79ff` 後の作業分）:
- `mir_inst_str_val` / `MirBlock_inst_str_val`: pool String の clone を除去
  （emit 側呼出は全て read-only であることを確認。pool 汚染は fixpoint で検出可能）
- `mir_inst_result_type_at`: 中間 Vec<MirValueType> 確保を除去し列直読化

検証: keepclock flat-src で s5-noclone をビルド — `wasm-tools validate` OK、
self-fixpoint byte-identical（sha256 `5f0f9470…`）、旧 compiler (s3-kc32) vs s5-noclone で
fixture 3 件（vec_generic / clone_independent / ord_sort_vec_string）出力 byte-identical。
`--time` 計測で emit.code 2.64s→2.20s（str_val clone 除去分）。
- `emit_f32_to_f64`: 共有 handler の方向判定を callee 名比較から result
  `val_type`（VT_F32/VT_F64）比較へ変換 — string-dispatch → 型駆動の初例。
  numeric_explicit_conversions / arithmetic_edges の新旧コンパイラ出力が
  byte-identical、s6-vt self-fixpoint `b61cb055…` で確定。

禁止: SoA 保存 + fat API、tick 173/178 級の再構築、`MirFunction` への場当たり field 追加。

## Phase 3 — whole-module 本体 MIR を廃止する

悲観 gate: **同時生存 body MIR ≤ 2 関数、RSS ≤ 384MB、wall ≤ 35s**。
35 秒超過は 10 秒計画の赤信号。GC をいじる前にアルゴリズムを再調査する。

- [ ] discovery: FunctionKey / TypeKey / signature / closure / mono worklist。本体 MIR なし
- [ ] body: 1 関数 lower → 局所解析 → Wasm body → reloc 保存 → arena reset
- [ ] CALL / GC type は FuncId / TypeId reloc。最終izer が数値 index にする
- [ ] 「index 未定だから全 MIR 生存」を消す

residency map（2026-10-12 調査 — なぜ全 body が同時生存か）:

1. `lower.body_emit` が全 decl 本体を共有 `MirModule`（body_store: inst/local/
   body の巨大 Vec、592k insts / 291k locals 生存）へ書き込む
2. `closure_flush_pending_fn_thunks` + `gc_late` が decl loop 後にさらに body を追加
3. `reachability` prune が module-wide call graph（body 内 edge を辿る）
4. `sync`/`propagate`/`mir_opt` が module-wide scan
5. `emit.code` が全 body を反復して wasm byte 化 + 約 31 箇所の全関数 scan
   （signature/export/indirect table/debug map — body ではなく metadata を読む）
6. `emit.tail` で section 確定

body MIR が本当に必要なのは 3・4 の一部と 5 の body emit のみ。
signature/export/indirect/debug/p2 wrapper は metadata 消費で、
discovery index（fn_index + signature_registry + mono view）で賄える。
`ctx_call_edges` は既に edge table として存在 — reachability の
「body 走査で edge 発見」を「lower 中に edge 記録」へ転換すれば、
body の生存理由は「自分の wasm body byte 化まで」に限定できる。

Phase-3 手順の設計:
a. discovery 完了形を固定: fn_index（名前→decl/typed index）+ signature
   registry（func_id→sig+core_op_id）+ mono index を body 非依存で構築
   （現在 fn_index 5.5s / ctx_prep — body CoreHIR は mono worklist 経由のみ）
b. per-function loop: decl 順または worklist 順に body lower → per-fn
   sync/propagate（post-pass の関数化）→ wasm body byte 化 → reloc
   記録（call site → callee FuncId、gc type use → TypeId）→ MIR 破棄
c. reloc finalizer: emit.code の function_idx が事実上の FuncId 位置
   （module position = wasm func index + import offset）。call site は
   `call <func_id>` reloc を書き、section 組立時に index 解決
d. edge 駆動 reachability: callee 未到達なら worklist へ追加 —
   「index 未定で全 MIR 生存」を消す本体

未決: body emit が必要とする signature 以外の情報（struct field layout、
closure env、capture 解析）は discovery で TypeKey/closure として確定する
必要あり — ここが body 非依存化の実質コスト。最初の実装単位は
「edge table を reachability 入力にする」+「per-fn emit 直後の body drop」
の順で、whole-module 保持のまま先に edge-table 化を進める。

既存基盤の棚卸し（2026-10-12 Phase 3 着手前調査）:

- reachability は既に edge 駆動: `ctx_edge_table`（lower 中に CALL/REF_FUNC の
  fid+name を packed stream 記録）→ `mir_build_recorded_edge_lists` で
  per-fn callee list 化 → BFS。recorded edge がある fn は body scan を
  スキップする — 既に「index 未定」回避の片側は済み。
- call reloc は既実装: `SelfEmitCtx_push_call_reloc(slot, fid_raw)` +
  placeholder `emit_leb128_u_fixed(0)` + `apply_call_relocs` で
  finalize 時に index 解決。`resolve_fn_index_for_call` は fid→mir 経由を
  優先（`ctx_fid_map` の name-chain map 経由）。
- per-fn body detach は既実装: emit.code loop が body 出力直後に
  `mir_function_detach_blocks(f, mir)`（module_block_pool で block 再利用）。
- `callee_prop_cache` は signature 級（fn name + return_type_name のみ、
  body 不読）→ registry source 化すれば pre-emit propagate 可能。
  2026-10-12 実装済み: \`MirModule.return_variant_slots\` — RETURN local の
  variant slot を fn push 時点で記録（ctx_push_{module,current}_function）。
  prune は並列表を併せて filter、非 ctx 経路の module は sentinel -2 →
  旧 body scan fallback。callee cache build は body を一切読まなくなった
  （name + return_type_name + slot 全て metadata）。37 fixture byte-parity、
  verify lane PASS。残る body 非依存化: by_name/by_bare の fn 名 scan は
  signature_registry への切替候補。
- sync/propagate は per-fn 独立（共有は read-only TypeTable + cache）—
  decl loop 内 per-fn 化は「cache が全 fn 名を持つ前提」で阻害:
  forward-callee の return_type が未 emit だと lookup miss →
  伝搬が不完全になるため、cache を signature_registry 由来に直す必要。
- type index は eager 解決（reloc なし）: body emit は emit.types 後なので
  index 確定済み。late 登録は `ctx_ensure_gc_enum_payload_variant_typed` の
  enum payload（Option::Some 等、body 由来）で gc_late=2 — per-fn emit 化に
  は「type 早期凍結（discovery で全型登録）」か「TypeId reloc」のどちらかが必要。

残る実装 = pipeline 再構成の本丸: per-fn「lower→analyze→emit→detach」への
decl loop 書き換え。peak RSS（lower 終端で全 body+type 生存 ~600MB）を
下げるには emit を lower loop 内に移す以外に近道がないことを確認。

Phase-3 実測（2026-10-12 edge probe、`ctx_corehir_probe.ark` —
`mir_probe_edges_enabled()` を 1 にして rebuild すると [probeA]/[probeK]/
[probeR] が stderr に出る）:

- **実 body source は CoreHIR forest ではなく AstNode**:
  `mir/lower/input.ark` で `lazy_body=false` 固定、decl loop 中の
  `mir_entry_exprs` は空・`root=-1`。実体は
  `MirLowerEntryFallbackBody` → `MirLowerLegacyBody.node`（AstNode、
  `mir_legacy_decl_body_at` 経由）。「CoreHIR 読みの edge table 化」は
  前提が誤り — pre-lower edge 発見は **AST 走査**として設計する。
- **edge fidelity の実測（vec_option_i32: AST 125 vs recorded 229、
  caller mismatch 13）**: recorded stream は alias/mono/method 解決済み名
  （`push→vec_push`、`len→raw_array_len`、`Vec::new→Vec__new__i32`、
  `m:to_string→to_string`）を持ち、AST 側は生名＋非 call 構文
  （`Some`、`Box::Owned`、param `f`）を含む。AST edge table を
  recorded と同等にするには同じ解決レイヤが要る:
  builtin alias は `hashmap_builtin_alias` で再現可能、method→実 fn は
  レシーバ型が前提のため signature/type 情報を discovery で先に確定する
  必要がある（mono worklist 設計と一直線）。
- **`fixpoint --build` は検証ゲートに使えない**: wasm32+p1 stage-3 が
  既知の `SELFHOST_COMPILE_TIMEOUT >3600s` で止まる（旧バイナリでも同じ、
  環境起因）。fixture byte-parity が代替ゲート。

Phase-3 実測（2026-10-12 worklist skip / gc-late probe）:

- **worklist skip は着地済み**（`5bae97401`）: `mir_ast_edge_reachable_names`
  が AstEdgeTable を root（main/_start/__test_main + export_roots）から BFS
  し `emit_keep` NameIndex を生成。`mir_emit_view_decls` は emit 名が全て
  未到達の decl/method の `*_mono_bodies` を skip する。`entry_prefilter`
  （文字列名ベースの下位近似）は削除。MIR 名の跨モジュール衝突
  （`json::parse`/`parser::parse` → `parse`）対策として
  `AstEdgeTable.caller_lists` が同名 caller 全 position を保持する。
- **検証経路の注意**: `arukellt-selfhost.sh` の wasm 探索順は
  `$ARUKELLT_SELFHOST_WASM` → pinned → s2-runtime → s2 の順。素の
  `compile`/`run` は **pinned** を使うため current source の変更を検証
  しない。変更後の直接検証は
  `ARUKELLT_SELFHOST_WASM=$PWD/.build/selfhost/arukellt-s2-runtime.wasm`
  を export して行う（fixture parity は内部で current s2 を使うので影響なし）。
  s2 JIT 実行の flat-src 全体 compile は >5min なので、probe/計測用には
  `wasmtime compile … -o s2-probe.cwasm` で AOT 化すると小規模入力が
  秒で回る。
- **gc_late の実測（pinned --time）**: selfhost 規模で structs=0 enums=3、
  fixture 級では全件 0。late variant は sealed と同名で **sig だけが違う**
  （例: `Option::Some`/`Result::Ok` の
  `SubF+14_S_f0_i32_f1_ref22` — payload が `Vec<i64>` 等の concrete
  ref）。sig が payload 側 canon 型 index（`refN`）を内包するため、
  「enum 個数だけ先に凍結」では解けず、(name,sig) の完全列挙か
  canon registry の append-only 化か TypeId reloc が必要。
- **canon 型 index は base+slot ではない**: type section は canon sig
  dedup 順（`TypeCanonPlan`）で並び、`*_type_map` が slot→canon_idx
  を引く。late 登録が新 canon 型を足すと downstream（struct/callback/
  fn sig/wrapper/p2）の index が全てずれる — per-fn emit の前提は
  「全 canon 型の確定」か「型オペランドの reloc」に限定される。
  GC 型オペランドの解決点は `gc_struct_type_map`/`gc_enum_variant_type_map`
  参照の 19 ファイルに散在（struct.new/cast/get/set、indirect、vec、
  option/result intrinsic）— 全置換の reloc は大掛かり。一方で
  registration→index 即時確定の append-only canon registry は subtype 順序
  制約（super 先行必須）と現行 section 並び替えの互換を調べる設計論点あり。

Phase-3 設計確定（2026-10-12 canon 調査 — **TypeId reloc は不要**）:

- **type section の前方参照は合法**（実測確定）: GC/function-references
  型索引空間は section 全域で確立される。実 emit 済み module（
  late_probe.wasm）に type 30→32 等の前方参照が存在し、
  `wasm-tools validate --features all` が通る。`emit_type_section` の
  「Option/Result payload が gcsref で前方 index を踏まないよう struct を
  enum より先に置く」コメントの制約は実は不要だった。
- **よって canon 順序は自由**: 全 discovery 決定型（host/wit/gc_fixed/
  callback/fn sig/p2/linear/pair call）を discovery 完了時に固定順で canon
  化し、body 駆動の module-gc sig（enum payload variant・遅延 struct）は
  **登録順 append** で canon_idx を即時確定できる。late 登録は末尾に
  足すだけで既存 index を一切ずらさない — 早期凍結（集合の完全列挙）も
  TypeId reloc（19 ファイルの operand 書換）も要らない。
- sig 本文の `refN`/`gcsrefN` は canon index ではなく fixed 型番号と
  gc_struct registry slot を指す（`ctx_gc_enum_sig` 実測）— sig は
  registration 時点で完結し、canon 化時に参照解決は emit 側の remap に
  任せられる。
- 実装方針: ctx/module に append-only canon 表（sig→idx dedup + idx→sig）
  を持ち、discovery 末端で非 body 型を一括 ensure、各 registration path
  （`ctx_register_gc_enum_variant`/`ctx_ensure_gc_struct_registered`）が
  ensure で canon_idx を即時確定・保存。`*_type_map` は保存済み
  canon_idx の直読みに置き換わる。emit 順序は変わる（dedup 後の登録順）
  が valid かつ deterministic — fixture parity（output 一致）と fixpoint
  で検証可能。

実装順序の再評価:
a. discovery で edge/type を確定（**AstNode 走査の edge table 化** — MIR 不要。
   ただし recorded 同等の解決名にするため alias/mono/method 解決レイヤが前提）
b. per-fn post-pass: callee cache を signature_registry 由来に変換し、
   sync+propagate を decl loop 内で per-fn 実行（forward-callee の
   return type を registry が保持するため）
c. per-fn emit: type index の解決を「upfront 凍結で済む経路」+「late は
   TypeId reloc で placeholder+finalize patch」に分離
d. worklist 駆動で到達不能 fn の lower 自体を skip（~8% body_emit 節約）

## Phase 4 — post-pass type propagation を消す

gate: **lower + 全 dataflow + reachability ≤ 3s、whole-module instruction scan = 0**。
全体 15–20 秒を狙う。

- [ ] typed-by-construction。lower で決まる TypeId はそこで確定
- [ ] LocalId 更新 → `users(LocalId)` worklist だけ再評価
- [ ] enum / open variant は型 lattice の join。String 名推測をやめる
- [ ] reachability を FunctionKey queue にする
- [ ] `full typed sync` / module-wide propagate / 反復 body scan を削除

## Phase 5 — phase arena / GC を仕上げる（旧 #850）

gate: **peak RSS ≤ 256–384MB、GC wall ≤ 0.7s、total ≤ 12s**。
12 秒未達ならここで初めて collector 改良を検討する。

- [ ] function lower / analysis / emit scratch を関数ごとに reset
- [ ] durable: 最終 Wasm、TypeId 表、FuncId 表、reloc 表だけ
- [ ] `#850` の製品 arena をこの phase で実装する
- [ ] `#850` の tick 履歴は閉じた扉の証拠として残す

## Phase 6 — 最後の 2–5 秒だけ通常の最適化

12 秒を切ってから解禁。候補: AOT、bounds check、hot hash、string intern、
byte buffer、LEB、generated lookup、branch layout。

- [ ] 12s → 8s を profile-guided で取る
- [ ] 10 回計測で median ≤ 7s、p95 ≤ 10s、RSS ≤ 512MB
- [ ] `s2 == s3` + validate + fixture parity
- [ ] 受入後にだけ `BOOTSTRAP_EMIT_*` を wasm32-gc へ翻す
- [ ] `#851` を close する

## 真の停止条件

- 必須 tool / artifact が無く、リポジトリ内で再生成できない
- Phase 3 gate が構造的に不可能であることが receipt で示された
- ADR-053 を SUPERSEDE する新しい ACCEPTED ADR が出た

commit、tick 完了、1 回の overlay 成功、hello バイト一致、`#850` close は停止条件ではない。
- **`fixpoint --build` は検証ゲートに使えない**: wasm32+p1 stage-3 が
  既知の `SELFHOST_COMPILE_TIMEOUT >3600s` で止まる（旧バイナリでも同じ、
  環境起因）。fixture byte-parity が代替ゲート。

Phase-3 実測（2026-10-12 第2弾 return-table probe、同じ switch で
[probeT]/[probeT-miss]/[probeT-extra] が出る）:

- **fn_index は全 MIR fn を正規化済み return 名で 100% カバー**:
  `ctx.fn_index`（name→return_vt+return_type_name、ctx_prep で構築、
  `ctx_register_fn_signature` で lower 中に synth 名も追加）の全エントリに
  対し `ctx_maybe_enum_return_type_name` 適用値と
  `MirFunction_return_type_name` を比較 — vec_new_option_i32 で
  **0 mismatch / 0 extra**。`callee_prop_cache` の name+ret_name を
  fn_index 由来に切替可能で、forward-callee も pre-body に解決できる。
- **258 miss は emit されない intrinsic/prelude decl**（`clone`、
  `String_from` 等）。lookup 的には dead だが、`by_bare` では bare 名の
  ambiguity が増え得る（`clone` 汚染は #730 で既知）。by_bare は
  「emit される decl」のみに絞るか two-tier が要る。
- **ret_variant_slot は値フロー由来で signature からは供給不可**:
  `mir_function_return_variant_slot` は RETURN local の variant_slot を
  body 走査で読む。per-fn pipeline では pushed 済み fn のみ供給、
  forward-callee の dst は slot=-1 へ劣化する。callee の返す variant は
  caller 内 dataflow では再現できないため、enum payload 直アクセスの
  hint を失う分だけ emit code が重くなる可能性（正しさには影響なし）。
  callee-first emit 順なら pushed 済み slot が使えて回避可能。
- **`fixpoint --build` は検証ゲートに使えない**: wasm32+p1 stage-3 が
  既知の `SELFHOST_COMPILE_TIMEOUT >3600s` で止まる（旧バイナリでも同じ、
  環境起因）。fixture byte-parity が代替ゲート。

Phase-3 実装済み（2026-10-12、コミット順）:

1. `c3ff1f92e` `MirModule.return_variant_slots` — RETURN variant slot を
   fn push 時に記録。callee cache build の body 走査を除去。
2. `e829d01b5` edge probe（`ctx_corehir_probe.ark`、[probeA]/[probeK]/
   [probeR]）。source-level と recorded edge の fidelity を実測。
3. `c994a0b09` probeT 追加 — fn_index が全 MIR fn の正規化 return 名を
   100% カバーすることを証明。
4. `ea51ca0b4` `CalleePropCache` に discovery tier（fn_index 名 + 正規化
   ret 名、module に保持）。tier1 miss 時のみ tier2 exact lookup。
   10/10 fixture byte-parity。
5. `d49a15221` `ctx.prop_cache` + push 時 tier1 増分 + per-fn
   sync→propagate→sync。`ctx_register_fn_signature` は discovery 表と
   cache disc index にも追記。post-loop の module pass は収束 mop-up
   として残置（各 body を二重走査する過渡コストあり）。
   16/16 fixture byte-parity、verify lane PASS。

prefilter 前例の検証（2026-10-12）: `entry_prefilter.ark` は存在するが
事実上 dead — flat な ident/path 名マーキングは caller→callee 構造を持た
ず、mangled/lower 生成名を取りこぼして「overlay reachability=1」に
壊れたため `Always emit` で無効化されている。worklist skip (item d)
には **per-caller edge 表 + 名前解決レイヤ** が必要で、これは item (a)
の AstNode edge table と同じ部品。mono 解決には `ctx.mono_call_*` の
span→target rewrite map（frontend 由来、ctx_prep 済み）がそのまま使える。
