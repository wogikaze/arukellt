# セルフホストコンパイラ中核再構築

Status: active — Phase 3 (function-at-a-time) is the remaining gate  
Owner: overlay / #851 / ADR-053  
Created: 2026-09-01  
Last updated: 2026-10-29

決定: [`docs/adr/ADR-053-selfhost-compiler-core-rewrite.md`](../adr/ADR-053-selfhost-compiler-core-rewrite.md)  
追跡: [`issues/open/851-selfhost-compiler-core-rewrite.md`](../../issues/open/851-selfhost-compiler-core-rewrite.md)

この文書が **wasm32-gc + wasi-p2 overlay ≤10s** の実行正本である。
`#850` の tick 探索と、局所 hotspot 削りはこの正本に従わない。

## gc-host pin 復帰（2026-10-29）

`bootstrap/arukellt-selfhost.wasm` を wasm32-gc / wasi-p2 component pin
（`3c02d18b…`）に戻し、同日 2 回目の refresh で alloc-free matcher
（`85568bde0`）を焼き込んだ（`35a2585a…`、p1 identity `02bd6917…`、
gc self-reproduction `80469233…`）。refresh 後の実測は build-compiler
warm **28.2 s**（旧 core pin は 2011 s / 34 分）、fixpoint **29.2 s**、
s2 == s3 == `02bd6917…`。`BOOTSTRAP_EMIT_*` は wasm32 / wasi-p1 のまま据え置き。
残る ≤10 s 目標は Phase 3 以降で詰める。

同日 3 回目の refresh として #944 merge（`783be1b8f`）の source を
`62aecf93…` に焼き込んだ。memory32 / memory64 の gc self-build は
`wasm-tools validate --features gc,function-references,memory64` を通過し、
p1 identity `d917f7cd…`、gc self-reproduction `3ca4b66e…` を確認した。

### 現行床（2026-10-29、gc pin、clean `7438a59fb`）

| 条件 | wall | RSS |
|---|---:|---:|
| gc pin cwasm host（AOT、通常 overlay、wasm32-gc 出力） | median **39.1 s** / p95 39.6 s | 1.10 GiB |
| gc pin wasm host（JIT、通常 overlay） | median **40.7 s** / p95 42.1 s | 1.48 GiB |
| keep-clock gc host（phase 実測用） | median **39.9 s** / p95 40.2 s | 1.50 GiB |

refresh 後の pin（alloc-free matcher 入り）で build-compiler warm **28.2 s** /
fixpoint **29.2 s**（quiet、2026-10-29）。ユーザー目標
`min(0.3x, 30 s)` の build-compiler 側は満たした。gc overlay gate の
median は同一条件 A/B で 31.76 → 30.25 s（guest −8.9%）。

receipts: `851-gc-pin-overlay-aot-20261029.json` /
`851-gc-pin-overlay-wasm-20261029.json` /
`851-gc-pin-keepclock-20261029.json`。

phase 内訳（keep-clock、guest 計 ~35.9 s）:

| phase | s |
|---|---:|
| resolve | 4.3 |
| typecheck | 6.6 |
| lower.total | 24.3 |
| — lower.body_emit | 16.2 |
| — lower.inloop_emit | 5.7 |
| — lower.fn_index | 2.0 |
| — lower.edge_bfs | 1.1 |
| emit（types/code/tail） | 0.8 |

host の GC + startup が wall との差 ~4 s。10 s への主要ギャップは
**body_emit 16.2 s / typecheck 6.6 s / host GC ~4 s**。

phase 実測用の keep-clock gc host は、overlay を
`ARUKELLT_OVERLAY_EMIT_TARGET=wasm32-gc ARUKELLT_OVERLAY_KEEP_CLOCK=1`
で生成し、現行 gc pin で `--emit component` → `wasm-tools component
embed/new` で再パッケージする。runner も同じ env で実行する（通常 overlay
のままだと clock が stub され frontend phase が 0 になる）。

wall はホスト負荷に強く依存する（同一 host/source で quiet 31.8 s 〜
混雑時 39.9 s）。A/B は同一時間窓で baseline/変更版を交互または連続取得する。

### alloc-free qualified-name matching（2026-10-29、`407f2767e`）

gc overlay を `wasmtime --profile guest`（10 ms、3,618 samples）で分解:
`eq` 11.9% self、`MirModule_gc_enum_variant_matches` 6.4%（すべて
`typed_mvt_sig` から）、`name_index_find_slot` 5.9%、
`is_namespace_member_callee` 5.5%、`sig_reg_leaf_instance_name_match` 2.2%。

enum variant / callee spelling / sig-reg leaf の 3 matcher が per-call で
`"::" + name` を concat し operand を clone していたため、
`corehir::name_match` の byte-region 比較に置換した（alloc 0）。

A/B（同一 keep-clock host 条件、quiet、各 3 run、receipts
`851-alloc-free-name-match-{base,opt}-20261029.json`）:

| | wall median | guest median | body_emit | inloop_emit | edge_bfs |
|---|---:|---:|---:|---:|---:|
| baseline | 31.76 s | 28.93 s | 12.53 s | 4.55 s | 0.83 s |
| alloc-free | 30.25 s | 26.34 s | 11.31 s | 3.31 s | 0.62 s |

wall −4.8% / guest −8.9%。plan の wall 10% ルールには届かないが、
`inloop_emit` −27%・`edge_bfs` −25% が 3 run で安定し、allocation churn を
減らす Phase 5 と per-fn loop（Phase 3）の前提になるため残す。
次は `name_index` の probe 数（5.9%）と enum base index の O(types×variants)
を削る。

この matcher 変更は pin に焼き込み済み（`35a2585a…`、`22e403cbf`）。
焼き込み後の build-compiler は warm **28.2 s** / fixpoint **29.2 s** で、
ユーザー目標 `min(0.3x, 30 s)` の build-compiler 側を満たした。
続く index/hash 実験（`15606318d`: `MirModule` の gc struct slot index と
`name_index` の probe hash 強化）は同一条件 quiet A/B で
wall 30.62 → 32.09 s、guest 27.53 → 28.23 s と改善なしだったため
revert した（`fca9acf34`、receipts `851-gc-struct-index-{base,opt}-20261029.json`）。
diffuse な `eq` 呼び出しは個別 index では動かず、Phase 3/4 の構造変更が
必要という判断。

### 命令数 A/B と layout index（2026-10-29）

wall はホスト負荷で ±25% 変動する（同一 host/source で quiet 31.8 s 〜
混雑時 39.9 s）ため、AOT cwasm + `perf stat -e instructions:u` を A/B 指標に
した（同一条件 2 run の差 <0.01%、負荷 20–50 でも安定）。
`ctx_struct_layout_struct_index` を registry 全走査から incrementally 維持する
NameIndex に変えた版は **262.23 → 257.15 G instructions（−1.9%）**、生成 wasm は
バイト一致、gc host の自己再生成も byte 一致。10% wall ルールには届かないが、
O(N) scan の除去として同日の #944/#917 batch に同梱され履歴に残った
（`a7292c683` 系）。
落とし穴: field_count==1 の layout を field push 完了前に index へ入れると、
gc host が誤コンパイルされて自己コンパイルが spurious E0212 を出す。
分類は「fc != 1 は push 時、fc == 1 は field push 完了時」に固定する。

merge 後の HEAD（`a7292c683`、#944/#917 取り込み済み）で build-compiler
warm **55.6 s** / user 45.9 s（負荷 ~20、overlay 再生成込み）、fixpoint PASS
（s2 == s3 == `e6fbb279…`）。quiet 時の 28.2 s は #944 前 source での値。

gc struct slot index（`MirModule.gc_struct_slot_index` で
`MirModule_gc_struct_slot` の線形走査を O(1) 化）も同じ命令数 A/B で測った:
**248.28 → 239.28 G instructions（−3.62%、2 run で再現）**、overlay gc 出力は
バイト一致、gc host の自己コンパイルは自身の s2 `8fdebc10…` を再現、fixture 4/4
バイト一致。layout index（−1.9%）と合わせて名前走査の除去で累積 **−5.4%**。
単独では 10% ルールに届かないが、決定論的に効く O(N) scan の除去として残す
（2026-10-29 の `f1ce43311` merge に同梱された）。

並行セッションが同一 checkout で commit/push するため、s2 と fixpoint の
source がずれることがある。fixpoint は source 変更後に s2/s3 を同一 source から
再ビルドして評価する（`fixpoint` の cache を過信しない）。HEAD
`ba4811c9b` で再ビルドした結果: build-compiler warm **31.5 s** / user 30.4 s
（負荷 ~4）、fixpoint PASS（s2 == s3 == `152f13d9…`）、stdlib_json /
generics_v1 parity PASS。

### stub module に当たった失敗（2026-10-29）

`ctx_find_leaf_mono_instance_index` の O(M) scan を O(1) index にしたが、
bootstrap overlay は `_patch_bootstrap_stub_static_dispatch` が
`mir_lower_ctx_mono_type_params.ark` を **丸ごと stub 文字列に置換**するため、
変更は自己ホスト compiler に一切入らなかった（overlay を grep して判明、
命令数も baseline と完全一致）。revert 済み（`18e1d56a3`）。

教訓: bootstrap overlay で効かせたい変更は、まず overlay 生成結果を grep して
「その関数が実際に overlay に居るか」を確認する。stub 化モジュール
（`ctx_mono_type_params` など）の最適化は checks.py の stub テンプレートを
直す必要があり、bootstrap 専用の回避策なので手を出さない。

最新 pin `62aecf93…` と HEAD `55675414f`(+revert) の再検証:
build-compiler warm **59.8 s** / user 52.8 s（負荷 ~10、overlay 再生成込み。
quiet 比較ではない）、fixpoint PASS（s2 == s3 == `be07c165…`）、
stdlib_json / generics_v1 parity PASS。

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
- [x] CALL は FuncId reloc（既実装）／GC type は登録時 canon_idx
      （sealed/late 分割で確定、reloc 不要 — 後述「設計確定」）。最終izer が数値 index にする
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
  enum payload（Option::Some 等、body 由来）で gc_late=2 — 当初は
  「type 早期凍結」か「TypeId reloc」が必要と見積もったが、後述の
  sealed/late 分割により late 型も登録時に canon_idx が確定するため
  reloc は不要になった（2026-10-12 訂正版）。

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
  「enum 個数だけ先に凍結」では解けず、登録時 index は
  late region の登録順配置（sealed/late 分割）で解くことになった
  （下の「設計確定」参照）。
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

Phase-3 設計確定（2026-10-12 訂正版 — **前方参照は不可、sealed/late
分割で登録時 index 確定**）:

- **type section の前方参照は rec 群なしでは invalid**（バイナリ合成で
  実測確定: type0 の func param `ref 1` → `unknown type 1: type index
  out of bounds`。self-ref は暗黙 rec-of-1 で合法）。同日の awk probe
  が late_probe.wasm に「type 30→32 の前方参照」を見たとする記録は
  parse 誤り — Python 再スキャンで s2-runtime（263 types）・probe 群は
  全件 0 前方参照だった。よって module 型を collection 末尾へ移す設計は
  破棄 — fn sig / pair sig は `gcsrefN` で struct slot・__fnpair を
  参照しうるため前方参照になる。
- **sealed/late 分割で登録時 index が成立**: module ブロックは従来通り
  全 sig 領域より前に置き、内部順序だけを
  `[sealed_structs][sealed_enums][late（structs+enums 登録順 interleave)]`
  にする。late idx = late_base + 登録順、late_base =
  host+wit+fixed+callback+sealed 計で sealed-flush 時に確定するため
  **全 GC 型が登録時に最終 index を持てる**。sig 領域は late append で
  ずれるが body operand ではないため従来通り emit 時解決でよい。
- 参照制約の確認: late enum→late struct（__fnpair・payload field）は
  ensure が sig 構築より先に走るため必ず登録順が前方になる。
  late struct→late enum variant は存在しない（field は enum base
  `ref14` を指し、variant slot を指さない）。sealed 内は ctx_prep の
  deps-first reorder 済み。fn/pair/p2 sig は module ブロックより後なので
  常に後方参照。
- 残存リスク（現行 layout と同じ露出であり新規ではない）:
  `ctx_ensure_gc_struct_registered` の placeholder で parent slot <
  field slot になりうるため、late struct→late struct は稀に前方参照に
  なりうる（late struct が実質 __fnpair/tuple のみで未観測）。
  late struct が増えたら late region を deps-first topo にする。
- parity: late 登録なしの module では emitted order が現行と完全一致
  （全 fixture が対象）。selfhost でも late_structs=0 のため late
  block は enum のみ = 現行と同順序。順序が変わるのは
  struct/enum 混在 late のみで、そちらも valid。
- 実装（作業中）: `MirModule.{sealed_gc_struct_count,
  sealed_gc_enum_count, late_gc_order}`（packed slot*2+kind、
  module_gc_order.ark）、`LowerCtx.gc_seal_done`、登録点
  （`ctx_register_gc_struct`/`ctx_set_gc_struct_sig`/
  `ctx_register_gc_enum_variant`）で order 記録。emit 側は
  `gc_module_emit_slots`（sealed → late 登録順 → 未記録 slot sweep）
  を sig 発行順と slot→canon map の両方の正本にし、
  base+slot 算術を撤去した。次の作業単位: (C) registration path が
  canon_idx を登録時に即時確定して entity へ保存し、
  `*_type_map` 参照を置き換える。
- 実装状況（`e90d327f1` + `2a58dda63`、(C) 着地）: sealed/late emit
  順 + 登録時 canon 確定まで完了。canon ensure 順は
  `gc_module_emit_slots` と完全一致 — [gc_fixed][typed callbacks]
  [sealed structs][sealed enums][late 登録順] で、その後を
  wasi/wit/fn/wrapper/p2/linear/pair の canonicalize 分が埋める。
  fixed+callback prefix は初回登録時に lazy ensure
  （`ctx_gc_canon_prefix_ensure`、flag 冪等）。sealed flush は slot 順
  inline ensure、late は slot 順 push 後に `ctx_gc_canon_ensure_late`
  が late_gc_order 順で set（未記録は kind 別 sweep、emit tail と同形）。
  emit の `gc_struct_type_map`/`gc_enum_variant_type_map` は module
  保存の canon vec 直読みに置き換え済み。`MirModule_push_gc_*` が
  canon=-1 placeholder を並列 push し ensure で set する構造のため、
  per-fn fusion で登録点が body emit 時点に移っても同じ hook で確定
  できる。次の作業単位: per-fn「lower→analyze→emit→detach」の decl
  loop 書き換え（gcsrefN は emit 時点で canon_idx 直読み可能）。

- 実装状況（2026-10-13、module-wide sync/propagate 撤去）:
  per-fn post-pass は `ctx_run_per_fn_post_pass`（ctx_module.ark）として
  全 fn push ファネル（`ctx_push_{module,current}_function`）で
  sync→propagate→sync を実行済みだったため、entry.ark post-loop の
  `mir_module_sync_all_value_types`×2 + `mir_module_propagate_local_types_report`
  を削除した。cache は push 時 `callee_prop_cache_add_fn` で tier-1 に
  追加され forward callee は tier-2 discovery 名で解決 — thunk/mono
  も push 経路でカバー。47 fixture（enums/generics/collections/
  iterator/closure_capture/match_extensions）で emit wasm byte-identical
  を確認。残る本丸: body emit（wasm byte 化）の decl loop 内移設 —
  code section 順序のため body byte の fn index 順バッファリングが必要。

- 実装状況（2026-10-13、materialize-at-registration 着地）:
  post-seal GC 登録が登録時点で module slot push + canon_idx ensure +
  layout plan bind まで行うようにした。struct は placeholder sig
  （"GS"→ctx_set_gc_struct_sig 書換）経路があるため
  `gc_struct_sig_pending: Vec<bool>` を LowerCtx に追加し、
  `ctx_gc_struct_materialize_ready` が pending 直前まで slot 順に
  drain — canon 発行順を late_gc_order（=登録順）と完全一致させる。
  nested ensure（A→B→A 系）でも A 確定時に A,B 順で drain されるため
  baseline と canon order が変わらない。enum は sig 確定登録のため
  push 分岐で即 materialize。両 flush_from は module 保有数から resume。
  TypeId 解決 helper は新規 leaf module `mir/gc_layout_slot_bind.ark`
  へ移設（ctx_*→lower::gc_layout_plan_build→aggregate_field→ctx_* の
  import cycle 回避）。entry.ark は seal 直後に module-held plan へ
  fixed+sealed bind を実行し、post-loop build は冪等再実行として残す。
  139 fixture（enums/generics/collections/iterator/closure_capture/
  structs/generics_v1/match_extensions/memory64/mir_struct/
  struct_mutation/ownership/question_mark/regression/native_gc_stress/
  lang_uplift）で byte-identical。ついでに未使用 thin wrapper 2 件
  （mir_ast_edge_table_caller_count, MirModule_canon_count）を削除し
  先行していた ratchet drift（HEAD で 1754>1753）を 1752 へ解消。
  次の作業単位: SelfEmitCtx の ctx_prep 前移設配線と call_indirect
  の type reloc（sig→canon finalize 解決）。

- 実装状況（2026-10-13、per-fn body writer 着地）:
  declared body は per-fn `WasmWriter` へ emit し
  `SelfEmitCtx.body_bytes[func_idx]` に durable 保存、code section は
  保存 vec から func_idx 順に sized assemble する形に変更。linear-GC
  runtime index 設定は `code_configure_linear_gc_writer` へ抽出して
  code_sec（synthetic bodies）と各 body_w に適用。166 fixture
  byte-identical。body 内 call reloc の offset は body-relative に
  なった（apply は依然 body 完了時 — callee fn_idx は discovery 時点で
  確定済みのため in-loop でも即時解決可能）。

  **in-loop emit readiness 監査**（body emit が読む post-loop 限定値）:
  - `ctx_string_view` 系: 通常 literal は `ctx.string_offsets` 経由だが
    値自体は `DATA_START + MirModule_string_data_rel_at` と同一 —
    module SSOT 直読みへ切替可能（ctx に module 参照 or rel vec の
    配線が要る）。bool literal（"true"/"false"/"assertion failed"）は
    plan 時 tail append のため **string reloc 必須**。
  - `fn_type_map` / `pair_call_type_map`: call_indirect 系で参照。
    canon order 維持のため push 時 ensure は不可 — **type reloc**
    （func_idx + body_rel_off + 解決 key を記録、finalize で patch）。
  - reloc は body_bytes[func_idx] 上の body_rel_off を固定幅ではなく
    **splice patch** とする（i32.const 等の minimal SLEB を維持し
    byte-parity を保つ）。同 body 複数 site は offset 降順 apply で
    位置ずれを防ぐ。

- 実装状況（2026-10-13、ctx_reloc + body buffering + call_indirect reloc 着地）:
  3 commit で emit 側の fusion 前提を整備した。全て byte-identical 検証済み。
  - cca998b45: declared body は per-fn WasmWriter へ emit し
    SelfEmitCtx.body_bytes[func_idx] に durable 保存、code section は
    保存 vec から func_idx 順に sized assemble。emit phase / assemble
    phase の分離点が確立（166 fixture）。
  - fdde5b57b + e30334b42: ctx_reloc — placeholder + (func_idx,
    body_rel_off, kind, keys) 記録 → assembly で minimal-encoding splice。
    bool literal（STR_TRUE/FALSE、GC/linear 両 path）と panic message
    （STR_VALUE、iov/p2 write）を consumer 化。memory64 でも真 i32 の
    operand は emit_i32_const_true_reloc で分岐（131+91 fixture）。
  - a6401fd53: call_indirect type operand を reloc 化。kind は再解決
    型（PAIR_KEY=skey hint 文字列、INDIRECT_SCAN=arg_count/dest_vt/
    first_arg_vt）で apply 時に同じ resolver を再実行 — map index
    保持より確実。MIR arg1>=0 は直接 emit。apply は import cycle
    回避のため code.ark の code_apply_pending_relocs（130 fixture）。
  - f03044b03: SelfEmitCtx が mir: MirModule を保持（module SSOT への
    入口確立）。plan 由来の string_offsets/lengths/values 3 vec を廃止し、
    ctx_string_view は DATA_START + MirModule_string_data_rel_at と
    MirModule_string_constants 直読み。intern 時点で rel 確定のため
    plan 非依存 = in-loop emit で文字列 operand が解決可能に。
    EmitStringTablePlan から無読者の string_lengths も除去（97 fixture）。
  - ab1936764: gc_struct_type_map / gc_enum_variant_type_map の ctx 複製を
    廃止 — 全 15 reader が MirModule_gc_*_canon_idxs(ctx.mir) 直読み。
    setter の order-dependent rebuild 副作用は
    SelfEmitCtx_finalize_gc_derived_indexes へ集約（wasm_sections から
    gc setter 群の後に 1 回呼出）。facade の dead getter 2 件も除去
    （97 fixture byte-identical）。
  - c0d85e976: call_ref の ref.cast / call_ref 両 operand を reloc 化。
    emit は (arg_count, dest_vt, callee_local, raw_hint,
    callee_type_name) を記録し、apply で resolve_call_ref_type_idx_after_local
    を再実行 — func-scoped local probe は emit 時に chain 位置そのまま
    inline 化し、hit なら直接 emit / miss なら reloc。pair_key/pair_sig
    map を含む post-loop 非確定値が call_ref 経路から除去（97 fixture
    byte-identical、gc-layout-audit カウンタも一致）。
  - 2c315f237: call reloc を module-level 化。(func_idx, body_rel_off,
    fid_raw) を記録し emit_code_section で apply — 5-byte fixed slot
    を reloc_patch_leb_u_fixed で in-place patch するため splice 系
    reloc より先に適用（splice は引き続き逆順）。per-body の
    call_reloc_used pool/reset は廃止。thunk fid のような mid-loop
    登録 fid への forward ref 経路が確立（97 fixture byte-identical）。
  - 2e006f117: indirect_type_returns_value を module canon sig 直読み化。
    type_idx → canon_sigs[type_idx] の末尾 '_' セグメント（'0'=void,
    '1<tok>'=値あり）で arity を判定し、plan 由来 fn_type_map scan を
    廃止 — store 形状判定が in-loop emit で利用可能に（97 fixture
    byte-identical）。
  - b4aae6172: SelfEmitCtx を emit_type_section 前に構築 — ctor +
    module 由来 setter（gc names/sigs、signature spine、gc_layout_table
    handle）を早期化。type_plan 由来 setter のみ post-section に残る
    純粋なスケジューリング変更（97 fixture byte-identical）。
  - 31a8790fb: call type 解決を module canon 読み化。emit_fn_type_idx
    = canon_index_of(build_function_type_sig(fi)) で fn_type_map 相当を
    module から読み、call_ref_pair_key / fnsig finder /
    resolve_indirect_type_idx / memory64 callback finders が plan 非
    依存に。pair_call_type_map ctx field 廃止、pair_call_sigs は
    ctx_prep で収集（97 fixture byte-identical）。
  - fc61b0a99: gc_type_base / gc_struct_base / gc_enum_variant_base を
    module canon から導出（type_plan 不在で確定）。
    finalize_gc_derived_indexes も早期ブロックへ（97 fixture）。
  - 3e3a21271: gc_layout_table を ctx_prep で populate — emit_type_section
    から gc_layout param を除去。populate は module plan + gc base/canon
    のみ要する（97 fixture byte-identical）。
  - 0a2f96d54: fn_type_refs を lazy 化 — 新規 ctx_fn_type_refs が
    canon_sigs[type_idx] を parse して param/result ref idx を即答。
    ctx.fn_type_refs / ctx.fn_type_map field を削除し、SelfEmitCtx は
    type section 前に完全 populated になった（97 fixture byte-identical）。

  **残り in-loop emit ブロッカー**:
  - decl loop 融合本体（emit_function_body を mir_emit_view_decls
    内へ移設）。調査（2026-10-13）で確定した前提条件:
    - SelfEmitCtx を MIR lowering pipeline で構築する配線（ctx.mir は
      構築中の module を共有 — functions/canon_sigs は push/ensure で
      育つ参照型）。
    - fn 解決の spine 化: fid_to_mir / function_name_counts /
      functions[] / type_name_ref_index は post-lower 完全 module 前提。
      融合では callee が未 lower の forward ref が発生するため、
      signature_registry（ctx_prep populated）+ name→decl 表で
      fid→decl_idx を解く。MirFunction_* の callee 読み（param_count
      等）は spine 経由に置き換えが必要。
    - pair_call_sigs: 現状 ctx_prep で全 MIR body を走査して収集するが、
      融合では body は未存在 — discovery edge walk から収集するか、
      loop 内で発見して post-loop 一括 canon_ensure（登録順維持のため
      即 ensure は不可）し operand は reloc 委譲。
    - writer_note_ref_top / ref_top_is の bookkeeping は emit 時に
      確定 type_idx を要する — reloc 経路では -1 になり得るので、
      cast elision を symbolic（未解決は elide しない）にするか
      解決済み key で記録する設計が要る。
    - strings は MirModule.string_data_rel が append-only で育つため
      live 読みで対応可能（ctx_string_view は既に module 直読み）。

  融合 v1 の実調査で追加確定した副ブロッカー（2026-10-13 第2調査）:
  - emit-spec plumbing: emit_function_body が要する wasi_version /
    memory64 / source_file / bindings(WitImportBinding) / opt_level /
    report_timing は MirLowerInput に無い。driver/lower.ark は config
    を持つので emit spec record を session → mir_lower_input へ流す。
  - mir_opt が lower と emit の間に走る（async_lower /
    stdlib_resolve_validated / O2 gc_hint — 全て内部は per-fn ループ）。
    融合では post-pass 後・emit 前に per-fn 版を呼ぶ。
    stdlib_resolve の call-target 解決は forward callee に registry /
    discovery catalog 経由で耐える必要がある。
  - verify_mir は fn body を読む — detach は post-verify の emit 側
    sweep に残し、in-loop では detach しない（v1）。blocks pool の
    live 範囲は emit まで延びるが Phase 5 arena が本来の解放点。
  - reachability prune は functions[] を除去して index を詰める —
    body_bytes / reloc func_idx は pre-prune index なので prune が
    old→new map を module に記録し、emit 側で compaction + remap。
  - function_name_index / function_name_counts / fid_to_mir は
    functions[] scan で構築 — SelfEmitCtx が pre-loop に作られるなら
    per-push drain 化が要る。remap_colliding_short_names は完全な
    functions が要るので post-loop の finalize に残す。
  - linear_gc runtime func_idx は最終 fn_count 依存 — v1 は
    gc target のみ融合し、linear は従来 loop のまま維持。
  - emit_ctx handoff: SelfEmitCtx を MirModule field に置くと
    MirModule↔SelfEmitCtx の record cycle になるため、artifacts
    （body_bytes / reloc vecs / audit counters）を module の plain
    field 群として引き渡し、emit 側は従来通り ctx を構築して
    adopt する（in-loop ctx が artifact vec を参照共有すれば
    handoff は O(1)）。

Phase-3 設計（2026-10-13 body-emit 融合の依存分解 — emit_wasm_context_new
と strings.ark の実調査）:

- **string offset は reloc 不要**: `EmitStringTablePlan` の offset は
  `DATA_START + 4 + Σ len(prev)` で insertion order にのみ依存し、
  intern 時点で最終確定する（append-only、既存 offset は不変）。
  in-loop emit は running `next_data_offset` で即時解決できる。
  bool literal は `emit_string_layout_ensure_bool_literals` が plan 時に
  末尾 append するため、ctx_prep で module table へ先行 intern して
  offset を早期凍結する。
  → 着地（2026-10-13）: `MirModule.string_data_rel` + `string_data_next`
  を `MirModule_push_string_constant` で維持、`intern_string` は push
  経路へ統一。plan build は `DATA_START + rel` の SSOT 直読みに
  置き換え（47 fixture byte-identical）。bool literal の emit-time
  ensure から ctx_prep 先行 intern への移行は残タスク。
  → bool literal 結論: 先行 intern は string table 順を変え全 fixture
  byte-diff を起こすため却下。**string reloc**（operand emit 時に
  placeholder + (func_idx, offset, literal_kind) 記録 → finalize で
  plan 確定後の tail offset を patch）が byte-identical を保つ唯一の
  経路。対象 literal は "true"/"false"/"assertion failed" の 3 件のみ。
- **SelfEmitCtx**: `functions` / gc canon vecs は module の ref 共有なら
  push/ensure と共に育つ。signature_spine/registry/type_table/mono は
  ctx_prep 時点で populated。必要なのは ctx 構築を ctx_prep 後へ前倒し
  する配線と、late 登録を反映する読み経路（canon_idx 直読み済み）。
- **fn sig canon**: 関数 sig index は fn push 時 ensure で確定可能
  （gc canon_idx と同型、登録順=push 順で layout 不変）。function
  section / call.indirect 型は即時解決になる。
- **body byte buffering**: per-fn scratch WasmWriter で sized body を
  生成し `body_bytes[func_idx]` へ保存、post-loop で vec header +
  func_idx 順 concat + synthetic bodies（wrapper/p2/linear-gc）。
  offsets map は組立時に記録。call reloc は (func_idx, body内offset,
  fid_raw) へ一般化して section 組立で絶対 index 解決。
- **arena reset**: 現行 `mir_function_detach_blocks` は block 返却のみ。
  inst/local SoA 行も fn 単位で解放するには body_store の per-fn
  region（freelist or bump reset）が要る — Phase 4 の #850 arena と
  同一設計点。

Phase-3 実測（2026-10-13 AST edge-table 解決レイヤ — emit 同一解決の再現）:

- AST edge table は「AST が名前で書ける callee」だけでは emit の
  呼び先と一致しない。emit は `call_rewrite::mir_rewrite_call_target`
  で callee を書き換えるため、edge 側でも解決名を同じ形に展開する
  `edge_rewrite_targets`（hashmap alias/ctor/family、vec coreop、
  builtin intrinsic、raw_array_*）を resolution に通した。
- **receiver-type 環境**: `recv.m()` は emit が receiver local type で
  dispatch する（`mir_try_resolve_impl_method` /
  `mir_rewrite_hashmap_method`）。`HashMap<K,V>` の
  values/entries/keys は impl method が存在しない adapter-only で、
  bare spelling も mono instance も無いため under-keep → emit call が
  unreachable stub 化していた（hashmap_entries_values trap）。
  body ごとの let/param/`self` → local-type env を edge_collect_calls
  に通し、method call で `hashmap_family_for_type` →
  `hash_map::hashmap_<pair>_<m>` adapter・`HashMap_<pair>::m` pair
  owner・`Owner::m`/`InstOwner::m` impl spelling を候補追加。
  typed 候補は additive（bare は常に併記）なので誤型でも over-keep
  のみ。fixture は expected 15 行と一致。
- rewrite 展開の循環を spelling-level dedup で停止（
  `mir_builtin_method_to_intrinsic` の `Vec::new`→自身 self-map、
  alias ping-pong 対策）。hit パス後・rewrite 展開前に seen マーク。
- 残留 [probeE-ureach] は `__fnthunk_*` のみ（thunk は decl 外で無条件
  合成される false positive）。`__hm_if_*`/`__intrinsic_*`/`vec_push`
  等の miss は intrinsic/inline emit で body 不要。
- 残る既知の型推論ギャップ（bare fallback が埋めている）: tuple
  destructure の element 型、for 変数の element 型、field access、
  `?` の error 型。いずれも adapter 型名を要さず bare_index で
  解決できるため現時点では over-keep に留まる。

Phase-3 実測（2026-10-13 edge walk の allocation watermark —
iterator_query_consumers stall の根本原因）:

- 症状: `stdlib_trait/iterator_query_consumers.ark` の emit が s2 で
  >600s stall（100% CPU・RSS 平坦 ~160MB、stall 位置は print 間で
  漂う）。emit ロジックではなく **s2-runtime 内蔵 linear-memory GC**
  が原因: 関数 prologue が `alloc_since_gc >= 64MiB` で collect を
  発火し、mark フェーズ（wasm func 11100/11101）は
  O(live_words × descriptor list) の fixpoint 再スキャン — emit 中盤
  の最大 live 時に 1 回の collect が分数かかる。
- ピン留め bootstrap（Oct 10 版）は grey-worklist/page-index
  collector 改良コミット群（aabd7c16b 等、Oct 12）より前 — s2 が
  埋め込む collector は構造的に旧式で、新 collector は s3 にしか
  載らない。よってローカルでは s3 ビルドが旧 collector の壁で
  >3600s かかり fixpoint ゲートも実質不可 — **恒久対策は bootstrap
  更新か gc-host s2 経路**（gc+p2 では同 fixture が数秒で通る）。
- 当該 fixture の walk 総 allocation は (64MiB, 66MiB] — 閾値を
  66MiB に binary patch すると通ることで測定。対症として edge walk
  側の allocation を削り watermark 未達にした:
  `edge_collect_calls` を emit-name ごとの walk から decl ごとの
  単一 walk（shared segment + `edge_push_caller_segment`）へ再構成し、
  method-call ノードの receiver 型推論を span キーの per-decl memo
  （EdgeTypeMemo）で O(depth^2)→O(depth) 化、env/segment/memo の
  Vec を decl loop 外で再利用化（pop clear）、mono span レコード無し
  の多数ノードで `precise` Vec 確保を hash head ガードに移動。
  結果: 同一 fixture が未パッチ 64MiB 閾値で ~4s に（emit された
  wasm は削減前後で byte-identical、34549B）。
- 教訓: watermark 直下の fixture は微小な allocation 増で stall 復活
  しうる。edge walk の候補追加は alloc 量を意識すること。

実装順序の再評価:
a. discovery で edge/type を確定（**AstNode 走査の edge table 化** — MIR 不要。
   ただし recorded 同等の解決名にするため alias/mono/method 解決レイヤが前提）
b. per-fn post-pass: callee cache を signature_registry 由来に変換し、
   sync+propagate を decl loop 内で per-fn 実行（forward-callee の
   return type を registry が保持するため）
c. per-fn emit: 全 GC 型が登録時に canon_idx 確定するため body operand
   は entity 保存の canon_idx 直読み（TypeId reloc は不要）。call 側は
   既存の call reloc を継続利用
d. worklist 駆動で到達不能 fn の lower 自体を skip（~8% body_emit 節約）

Phase-3 実装（2026-10-13 fused emit 第1段 — pre-prune drain 着地、
97 fixture byte-identical）:

- `emit_inloop::emit_inloop_drain`（mir/lower/emit_inloop.ark）が
  reachability mark（prune の marks と同一引数で再計算）のあと・
  prune 本体の前に置かれ、pre-prune module に対して body emit まで
  一括実行する。kept fn の body は mark 位置で emit、prune 対象は
  空 body hole として vec を埋め、adopt 時に map で compact する。
  順序は driver の backend 順をそのまま再現:
  `mir_opt::optimize_module`（module-level pass — pass 順と検証集計を
  保持、artifacts enable はその後なので driver 側呼出は no-op 化）
  → canon seal（emit_type_section と同じ `build_type_sigs` +
  `canonicalize_type_sigs` を先行実行。canon_ensure は idempotent
  なので emit 側の canonicalize は lookup のみに退化し、type
  section は byte-identical）→ emit ctx 構築（gc base / spine /
  pair_call_sigs / gc_layout populate、true/false string idx は -1 で
  STR reloc 委譲）→ 全 fn を module 順に emit_function_body。
- handoff は設計通り: artifact vec（body_bytes / reloc tuple /
  call_reloc(+names)）を drain ctx と参照共有し、emit phase は
  `code::emit_ctx_adopt_inloop_artifacts` で `prune_old_to_new` map に
  従い body_bytes / reloc owner / call_reloc owner を compact・remap
  して adopt + audit counters 移し替え + emit loop skip + detach sweep。
- 有効条件は emit_mode ∈ {wasm, core-wasm, component} かつ gc
  target のみ（MirLowerEmitSpec、driver/lower.ark が config から
  session/input へ流す）。reachability_legacy 有効時は drain せず
  従来 loop を維持。
- pre-prune drain で発火した stale-index は ref.func 経路:
  `try_emit_ref_func_inst` は MIR_REF_FUNC の functions 名前 scan を
  即時 `defined_function_index` へ焼き込んでいたため、drain 位置で
  pre-prune index が確定してしまった。`RELOC_KIND_REF_FUNC`（uleb、
  skey=関数名）を新設し、in-loop emit 時は ref.func operand を名前
  遅延にして apply で post-prune vec 上の last-match scan を再現する
  （`code::resolve_ref_func_reloc`）。call operand は既存の
  call_reloc_names deferral で同じ問題を解決済み。
- `MirEmitArtifacts.prune_old_to_new` の記録、
  `gc_layout_table_drain_plan` の resumable 化、
  `SelfEmitCtx.fn_index_watermark` / `fn_index_drain`、
  fid<0 call の name deferral（call_reloc_names）が remap 経路で
  実動作する状態になり、in-loop 融合本体の前提機構が一通り検証済み。
- mir_opt bootstrap stub 制約: drain は namespace-level の
  `mir_opt::optimize_module` のみを呼ぶ（overlay では stub 実装、
  実装では orchestrate 経由 — どちらの build でも同一 entry が
  解決する）。per-fn `mir_opt::optimize_fn` API は in-loop step 用に
  確保（`mir_opt_optimize_fn` = async_lower → validated stdlib
  resolve → O2 loop/gc_hint の per-fn 合成）。
- 検証: 97 fixture の `.wasm` / `.code` / `.log`（stderr audit 含む）
  が parity baseline（`.build/parity/old`）と完全一致、
  `lower.inloop_emit` phase timer が `--time` で記録されることを確認。
- 残: decl loop 融合本体は未着手 — 「残り in-loop emit ブロッカー」
  節のうち forward-ref fn 解決の spine 化、ref_top bookkeeping の
  symbolic 化、per-push 収集（pair_sigs / name index）は依然として
  in-loop step の前提条件。
- per-push emit 前提の実測（2026-10-13 第3調査）:
  - signature spine は `fn_index_view`（decl loop 前の indexing pass）
    で全件 populate — 未 push callee も `signature_registry` の
    name→fid→entry（params/returns MirValueType）で body なしに
    解決できる。`spine_lookup::function_param_mvt` /
    `function_return_mvt` が既存の薄い facade。
  - `MirModule_push_function` は in-place push — ctx が functions vec
    を参照共有すれば push が live に見える。vec 置換するのは
    `MirModule_set_functions`（mir_opt async_lower/orchestrate と
    reachability_prune）のみ — 融合後は per-fn optimize_fn に置き
    換わるため ctx 再 bind は不要になる見込み。
  - 残る emit-time MirFunction 依存は call 経路の param0 type name
    （ref.cast 判定）、`emit_fn_type_idx`（canon sig idx）、
    `resolve_call_fn_ret_vt`（fid>=0 は registry path 済み）。
    fid<0 call は spine name 検索で真 fn と intrinsic を区別する。
- canon 順序の確定事実（2026-10-13 第3調査 — 「fn push 時 ensure」
  仮定の修正）: `build_type_sigs` の出力順は [wasi][wit][gc fixed]
  [typed callbacks][module gc sealed→late 登録順][**fn sigs**]
  [wrapper][p2][linear][pair sigs]。fn sig は全 module gc 型の後な
  ので、decl loop 中の push 時 ensure は (a) 後続 fn の body が
  登録する late gc 型より前に fn sig が canon を占有し、(b) prune
  対象 fn の sig が canon 枠を消費する、の二点で順序が崩れる。
  → per-push emit では fn-sig 依存 operand（call_indirect の
  `emit_fn_type_idx` 系）は全て reloc 化し、apply は name/fid →
  post-prune MirFunction → 既存 `build_function_type_sig` →
  `canon_index_of` で解決する（sig 構築の再実装を避け byte 一致を
  保証しやすい）。
- emit 時点の**決定**は reloc では遅延できない: `ref_top` bookkeeping
  （call_ref / writer_gc の重複 ref.cast elision — emit byte に効く）
  は callee の戻り gc 型 idx を要する。未登録型には idx が無いため、
  writer の ref_top を idx ではなく canon sig テキストのキーで保持
  する symbolic 化が必須（canon dedup により sig テキスト一致 =
  型一致）。callee fn sig → result segment → gc sig テキストの導出を
  spine 情報のみで行う経路が要る。
- 実装状況（2026-10-13、per-push pair_sig 収集 着地 — 97/97）:
  artifacts を module 生成時点（emit_spec gate）で enable し、
  `ctx_run_per_fn_post_pass` が push ごとに CALL_REF の raw tuple
  (owner idx, hint, vt) を `art.pair_ref_*` へ記録。drain の seal は
  record を push 順に kept フィルタして sig を derive — batch kept-scan
  と同一入力・同一順序であることを 97 fixture byte-parity で確認。
  **罠**: early enable により real `optimize_module`（orchestrate）と
  bootstrap stub（`scripts/selfhost/checks.py` の BOOTSTRAP_MIR_OPT_STUB）
  の双方が「enabled = opt 済み」と誤読して drain 内 opt まで skip して
  いた（wasm byte は偶然一致し、gc-layout typed lookup だけ増加 —
  audit counter が検出器として効いた）。`art.opt_done` を新設して
  「artifacts 有効」と「opt 消費済み」を分離。stub 更新を忘れると
  flat overlay ビルドだけ静かに挙動が変わる点に注意。
- 実装状況（2026-10-14、push 時点 emit 直前までの前提整備 — 97/97）:
  - `emit_inloop_drain` は `ctx.emit_ctx`（LowerCtx が live module
    handle 上に構築した shared SelfEmitCtx）を再利用し、
    `SelfEmitCtx_fn_index_drain` が pushed fn を watermark 増分で name
    table に fold、`remap_colliding_short_names` は post-fold に走る。
  - per-push `mir_opt::optimize_fn` 着地: `ctx_run_per_fn_post_pass` が
    sync/propagate の直後に per-fn pipeline を走らせ、drain は
    `art.opt_fn_count` の prefix watermark 以降の remainder だけを
    同一 per-fn pipeline で処理（module pass の double-opt 回避）。
    全 pass が function-local（module 読みは populated 済み spine
    registry のみ）で batch 等価。注意点: bootstrap stub の
    `stdlib_inline_fn` は callee を module fn vec 前方検索する
    cross-fn pass で、push 時点では未 push callee を inline できない
    — 97 fixture では不発だが stub 経路固有の潜伏差分として残る。
  - `emit_fn_type_idx` の in-loop 残り 3 箇所（code_locals own-fn
    param、inst_convert own-fn result、call_fallback_resolved の
    callee result_ref for ref_top）を sig テキスト直接 parse の
    symbolic 読みに置換（canon 未密封でも同一値）。
- probe 実測（2026-10-14、97 fixture に drain で全 kept fn の sig を
  MirFunction-built vs spine-reconstructed で比較）:
  - `__closure_*`/`__fnthunk_*`（65件）は signature registry に無く
    name→sig 再構成は空を返す。synth fn は registry 経路の対象外。
  - enum 系 return（79件）で `gcref14`（A=MirFunction）vs `gcref`
    （B=mvt_to_type_name 由来）が不一致 — mvt→name は enum spelling
    を失う。`MirFunction_return_type_name` は post-lower 正規化済み
    （例 "Result_i32_String"、"struct:VecIter<i32>:12"）で、
    disc_ret_names もそれと一致しない（closure は空、"option:i32" の
    ような正規化前の綴りが残る）。
  - NODISC 0 — `discovery_fn_names` は decl + `ctx_register_fn_signature`
    経由の synth を全網羅するので「この名前の fn が後で push される」
    の完全 oracle として使える（ret name の sig 互換性はない）。
  - → 設計転換: forward-call（emit 時点で fn_idx<0 の module fn call）
    を含む fn は drain emit へ defer（現行経路で byte 一致保証）し、
    per-push emit は全 MIR_CALL が backward に解決できる fn のみに
    限定する。note_ref_top/ref.cast/ret_vt の spine 完全再構成は
    正規化前名の不忠実リスクが大きく、defer 側の segment 数を
    削る改善として後で扱う。
- 実装状況（2026-10-14、per-push body emit 着地 — 97/97 byte parity）:
  ctx_run_per_fn_post_pass が per-fn opt の直後に emit_inloop を走らせ
  る（prepare → gate → push）。prepare は毎 push で gc flush・
  ensure_late・ectx rebind・fid chain 増分 fold・layout plan の
  incremental drain まで行い、push は art.body_bytes[mir_index] に
  emit 結果を直接書く（gate 不合格なら空 placeholder）。drain は EMPTY
  slot のみを従来経路で emit し直す。
  - gate (emit_inloop_fn_emittable) の安定条件: (a) CALL_INDIRECT /
    WIT_CALL / CALL_REF は常に defer、(b) MIR_CALL fid>=0 は fid map
    が未 bind なら defer、(c) fid<0 は resolve_fn_index_for_call 成功
    または discovery oracle 命中で defer（残るは真 intrinsic で
    fallback が時点不変）、(d) plan_pending > 0 で defer、
    (e) ref 系 local の storage-layout lookup が miss のものを含む fn
    は defer、(f) variant_slot >= canon map 長の local を含む fn は
    defer。
  - 判った flip 機構: emit 時点の「miss→drain で hit」参照が全ての
    乖離原因。wasm_ref_type_idx_for_local / storage_ref_type_idx_
    for_mir_value_type が late registration で未 bind の local を持つ
    fn を push emit すると、producer/copy storage infer の優先順位が
    変わり local decl の型が変わる（hashmap_i32_fn_get: open-enum
    (ref null 14) → variant (ref null 39)）。bound 済み entry は
    append-stable なので hit 側は不変 — miss のみを defer する。
  - 残留リスク（未観測）: callee 戻り mvt・enum/struct name index の
    miss→hit は未ゲート。97 fixture で不発なので defer 候補として記録。
  - 副作用で発見した latent bug は #921 起票（nested if を call 引数に
    すると arm が slot 配送・consumer が stack 配送になり invalid
    wasm。pinned bootstrap で再現、本変更とは無関係）。
  - 計測メモ: s2 build ~35s（fmt swarm と競合時は 579s まで劣化した
    実績あり — 並列エージェント作業時は worktree/ARUKELLT_BUILD_DIR
    分離を厳守する）。parity suite 97 件 ~19 分。
- 実装状況（2026-10-14、per-push arena reset 着地 — 97/97 byte parity）:
  emit 完了した関数の MIR blocks を emit 直後に block_pool へ返却する
  配線。push emit (emit_inloop_push) は byte 書き込み直後に
  mir_function_detach_blocks、drain の EMPTY slot emit は
  fi < art.opt_fn_count（ctx-pushed のみ）に限り同様に detach。
  late fn（ctx 非経由 push）は emit-phase sweep まで body を残す —
  それらには pair_ref / funcref evidence が無いため。
  - post-emit body reader 監査: emit-phase で module 関数の body を
    読むのは (a) collect_pair_call_type_sigs（type section 内蔵 +
    wasm_sections の ctx.pair_call_sigs 初期化）と (b)
    module_has_funcref_values（table/element section）の 2 箇所のみ。
    (a) は drain が pair_ref records + late fn body union で seal した
    art.pair_call_sigs を使う pair_call_sigs_for_emit へ置換、
    (b) は pair_refs scan で記録する art.saw_funcref_values flag で
    短絡する。reachability は ctx-push fn 全員に edge map mark があり
    body fallback を踏まない（call-free fn は scan が空を返すだけ）。
    mir/body_store.ark は旧 gc-overlay 時代の未配線 scaffold で
    現 MirModule のフィールドを参照しない orphan — 今回の配線は
    その serialize 設計ではなく pool reset で実現した（後始末は別途）。
  - 残存生存: deferred fn の body は drain emit まで生存（gate 通過
    分のみ早期解放）。MirModule_block_count/inst_count は detach 後
    の live 数を報告する（receipt の reachability 行で効果が見える）。
- 実装状況（2026-10-14、selfhost-scale hang の真因特定 — emit 系では
  なく producer wasm の線形 GC）: flat-src / n≈700+ forward-call
  stress が s2 で固まる件、wasmtime -W timeout の wasm backtrace で
  emit_inloop_drain → emit_function_body → core_op_shadow_observe_call
  配下の linear-GC collect が走っていることを確認。s2 runtime が内蔵
  する collector は #909 以前の旧版（全 record list を mark 毎に
  線形走査 + marked-only fixpoint 反復）で、O(marked words × records)
  の壁を踏む。producer binary 差別の実測: 同一入力で pinned 17.5s、
  s2（旧 GC）>15min、arukellt-s3-worklist（新 GC）60s 未満。
  - 同時に #909 chain arena の未回収欠陥を発見: page-table chain node
    が bump-only で reclaim されず、総 commit 数が ~4M に達すると
    proc_exit(122) で無診断終了する（s3-worklist の flat-src
    compile が ~17min で EXIT=122 — 観測済）。#922 起票。
    emit_linear_gc_rebuild_chains（collect 先頭で page table クリア +
    chain cursor を arena 基底へ reset + 全 record の page 再登録）と
    safepoint trigger への chain free pressure 条件を追加し、arena を
    live + 1 cycle churn へ境界付けた。churn 再現器は旧 emitter で
    exit 122、新 emitter で完走することを確認。
  - producer の鶏卵問題: 新 GC を内蔵した compiler は chain OOM で
    大きな compile を完走できず、旧 GC の compiler は quadratic mark
    で止まる。解消経路は fixed emitter を含む s2（pinned→flat-src
    build、batch emit 経路で旧 GC でも ~17.5s 級）→ s2 が emit した
    fixed-chain binary が大きな compile を完走 → bootstrap refresh
    （PROVENANCE の s3==s4 fixpoint 手順）で pinned を新 GC 世代へ
    更新する。
  - 2026-10-15 追加（73d5bfd2e / 9bfa2b280）: chain reclaim 実装後も
    flat-src compile は ~304MB で張り付き 60min 不完走だった。原因は
    2 層目の futile-collect livelock — chain band（live footprint が
    band 内だと commit 毎に全量 collect）と metadata band（band-hot +
    free-empty で同様に毎 commit 発火）。CHAIN_COLLECT_MARK /
    METADATA_COLLECT_MARK のヒステリシス mark を両 gate に入れ、
    band-width 分の進行がない限り refire しない形にした（meta 側は
    PRESSURE/8 = 512KiB の retry window で live-set dip への再試行余地を
    残す）。arena も metadata/chain とも 128MiB へ拡大（compiler-scale
    live set は数百万 record 級）。probe で確認: live_edge 3.6M 24.6s /
    meta_band 5M 26.4s / meta_edge 8.15M+churn 179s 完走（旧: >300s
    停滞）。真に飽和した live set は従来通り OOM exit で停止する。
    なお s2-runtime（memory64 heap-patch host）自体の flat-src compile
    は従来の quadratic mark で ~15min 超かかる点は変わらず、
    producer speed の問題であり emit 正しさとは別系。
  - 2026-10-15 追加（d103863c2）: #849 根本原因特定・修正。
    `infer_get_unchecked_elem_from_next_call`
    （wasm/code_ref_locals_infer_dest.ark、#730 由来）が
    raw_array_get_unchecked の dest temp に「ブロック内の次の CALL の
    最後の param の named-struct 型」を無条件で刻印していた。dest が
    実際にその引数へ流れるか検査していなかったため、直後に来る任意の
    receiver call（`SelfEmitCtx_target(ctx)` 等）で temp が poison され、
    `SelfEmitCtx_apply_call_relocs` の local 51 が (ref null 184) で
    declare されながら (ref null 10) の値を store → validate 失敗
    （func 118/5353/5357 は全て同一箇所）。修正は
    `call_result_is_last_call_arg` ゲート — dest（または staged copy、
    既存 infer_local_is_copy_of 経由）が次 CALL の最終引数の
    LOCAL_GET 源である時だけ推論を発火。回帰 fixture:
    tests/fixtures/target_capability/gc_get_unchecked_dest_decl.ark。
  - gc-host-v5 構築手順の記録: Oct-14 gc-host（旧 emit）で flat-src →
    wasm32-gc を emit すると decl bug を含むため、wasm binary を直接
    修復して動く producer を得た（/tmp/wpatch/drive2.py — local decl
    heaptype を producer 型へ patched-same-length 書換 + write-only
    scratch local は local.set→drop+nop）。重要: wasm32-gc の auto は
    memory64 を選び `component new` が canonical realloc で失敗する。
    実行可能な component には `--memory-width memory32` が必須
    （gc-host.core は 32bit memory で package 可能）。
    gc-host-v5（fixed emit + native GC runtime, patched binary）により
    flat-src→wasm32 emit (s3-final) と flat-src→wasm32-gc selfemit
    (selfemit-v5, no-patch validate 期待) を実行中。
  - 2026-10-15 追加（1388020de）: #849 第2欠陥（join temp）修正。
    `mir_begin_if_expr` は分岐型を知る前に result local を1つ割り当て、
    statement 位置の if でも各分岐の最後の stack ref を LOCAL_SET で
    join temp へ書き込む。`push(Vec<Vec<i32>>)` vs `push(Vec<i32>)` の
    ように無関係な concrete GC 型が混ざると、単一の concrete decl では
    両方を収められず validation 失敗（self-emit func 6879 / 旧 emit の
    func 6873 local 23、`edge_push_caller_segment`）。修正:
    `local_set_conflict_storage_type`（code_ref_locals_block_scan.ark）が
    LOCAL_SET 書き込み源の storage type を解決し、不一致なら decl を
    enum_open（全源が enum family: variant 同士に加え variant-vs-open
    も）か `(ref null any)` にする。typed read は access site で既に
    ref.cast するため拡大 decl は安全。`copied_non_string_storage_type`
    側にも同じ join を適用し、conflicted local の copy は first-write
    型ではなく anyref を返す。
    検証: repro_min/repro_edge と self-emit 全関数が validate 清浄化
    — 修正を載せた gc-host-v6（同じロジックを含むバイナリ）の
    flat-src→wasm32-gc 再 emit は patch 無しで zero defect。
    回帰 fixture: gc_if_join_mixed_ref_temps.ark。
    なお arukellt-s2-runtime.wasm で flat-src compile は linear-GC の
    旧ランタイム（Oct-9 pinned emitter 由来、#922 hysteresis 前）に
    埋め込まれ futile-collect で進まない — producer 速度の問題で
    emit 正しさとは別系（#922/#851 側）。

Phase-3 実測（2026-10-19 per-push sweep quadratics — wasmtime
--profile guest による guest wasm シンボル分解、receipt:
[932-per-push-quadratics-20261019](../research/receipts/932-per-push-quadratics-20261019.json)、
issue: #932）:

- **旧 host の guest 内訳が確定**: --profile guest で旧 gc host の
  コンパイルを分解すると、ctx_push_current_function（46.6% incl）→
  emit_inloop_prepare（41%）→ gc_layout_table_drain_plan（38%）配下の
  mir_gc_layout_plan_valid 系 accessor が leaf 時間の中心
  （mir_gc_layout_binding_type_id 19.5% self）。
  型検査 merge は犯行ではなかった（typecheck は guest の ~10%）—
  Oct-12 keepclock で見えた typecheck 63s は s3-kc32 の旧型検査コード
  自身の話で、現行 host の構造的ボトルネックは per-push sweep にあった。
- **除いた quadratic（全て append-only 構造に index/watermark を付与）**:
  MirGcLayoutPlan.index_by_raw（bind dedup O(1) + validator O(n) 化、
  per-push O(bindings^2) → per-push O(new bindings)）、drain の
  [plan_seen,count) 増分検証、ctx_gc_canon_ensure_late の確定済み
  slot skip、MirModule.gc_canon_version 駆動の derived-index
  rebuild gate（3 NameIndex rebuild/push を解消）、plan_pending
  rescan の version/base/pending-growth gate、reachability index の
  name->slot chain + ordinal counter 化（O(FxR) → O(F+R)）、
  HashMap 呼出 rewrite の starts_with prefilter。
- **効果**: guest プロファイル標本数は 6178 → 2695 → 2248（~64% 減）。
  同一ソースの wall は v28 ~121-128s → fixed host ~88-103s（host GC が
  wall の ~65-75% を占める領域に到達 — 次は allocation 量そのものを
  削る Phase 5 arena 系が最大レバー）。s2==s3 byte-identical、
  fixture 18/18 byte-parity、validate PASS。

Phase-3 実測（2026-10-19 第2弾 host 分解 + defer 国勢調査 — receipt:
[851-host-gc-breakdown-20261019](../research/receipts/851-host-gc-breakdown-20261019.json)）:

- **perf record による host 分解が確定**: wasmtime copying GC が host
  サンプルの ~76-83%（CopyingHeap::forward 31.7% + scan_field 20.4% +
  collect_increment 12.1% + GcHeap::index 10.2% + memmove 6.8%）。
  同一 flat-src main.ark compile の wall 比較: default 107.7s /
  gc-heap-reservation=8GiB 92.3s / **collector=null 25.8s** /
  collector=drc 138.6s。null が示す GC-free 天井 ~26s は guest
  プロファイル ~22.5s と一致し、**~80s が純粋に GC コスト**。
- **collector の構造（wasmtime 46.0.1 ソース確認）**: Cheney 型
  semi-space で heap capacity は ~2x live に自動フィットするため
  reservation（VA 上限）は collect 頻度をほぼ変えない。コストは
  collections x live bytes —— live set と総 allocation（~3.2GB、
  null 実行の RSS で計測）の両方を削る必要がある。drc は write
  barrier + 定期 trace で悪化（+29%）。host 側 knob は尽きた。
- **defer 国勢調査（emit_inloop_fn_emittable 一時 probe、probe 除去済み）**:
  push 評価 11284 回のうち emit-ok 4562 / **defer-fidmap 4605** /
  defer-storage 1766 / defer-resolve 333 / defer-disc 18 /
  defer-pending・variant・indirect は 0。push 時 emit の最大障壁は
  fid→mir_index の push 依存（callee が後から push されると unmapped）。
- **fidmap 解除の条件分解**: fid→mir_index は push 順が registry 非
  sig-only 名系列と一致すれば spine 構築時に ordinal で先読み可能
  （fn_index_drain の same-name chain 消費規則のシミュレーションで
  drain 時に検証できる）。ただし emit 側が callee の MirFunction を
  dereference する経路（infer_call_result_gc_type_from_fn_at 系、
  call_fallback_resolved、計 26 ファイル ~55 箇所の function_at
  consult）があるため、pred index だけでは不十分 — callee MIR 無しで
  drain 時と同じ推論結果を出す spine/prop_cache 供給が前提になる
  （Oct-12 probeT で fn_index 正規化 ret 名 100% カバー済み、
  ret_variant_slot は callee-first 順で回避可）。

- **fid 順序仮説は棄却済み（drain 末尾突合 probe、同日）**:
  fid_to_mir[raw] vs registry 非 sig-only ordinal = ok 1 / diff 11284 /
  unmapped 1069。push 順序と registry 順序はほぼ一致しない（mono /
  synth / late push の割り込み）。fid→mir_index の事前確定には push
  名系列を view から先読みするシミュレーション（= 設計済み
  discovery パス）が必須。ordinal ショートカットは使えない。
- **Vec push の alloc 仮説も棄却**: GC target の raw_array_grow は
  capacity doubling 済み（new_len > old_cap で 2x 再配置）。総
  allocation ~3.2GB は Vec copy ではなくオブジェクト個数由来 —
  削るには構造削減（Phase 3 live-set / Phase 5 scratch reuse）しか
  残っていない。


## Phase 4 — post-pass type propagation を消す

gate: **lower + 全 dataflow + reachability ≤ 3s、whole-module instruction scan = 0**。
全体 15–20 秒を狙う。

前提: post-pass が推測する型の「正解」を typechecker が持ち、node-keyed TypeId と
解決済み FunctionId として CoreHIR へ渡していること（#933 / #934、
[type-spine 計画](typed-mir-signature-registry.md) の Phase 2.5）。この handoff が
無い間は、下の post-pass を消すと別の名前推測が増えるだけになる。

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
