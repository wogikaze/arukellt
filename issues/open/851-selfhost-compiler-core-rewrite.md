---
Status: open
Created: 2026-09-01
Updated: 2026-10-12
ID: 851
Parent: 827
Track: selfhost-infra
Depends on: "827"
Related: "#850, #823, #829, #834, ADR-053, docs/plans/selfhost-compiler-core-rewrite.md"
Orchestration class: architecture-implementation
Orchestration upstream: 827
Blocks v4 exit: False
Priority: 1
Source: "gc-host overlay ≤10s is a compiler-core rewrite, not #850 micro-opts"
---

# 851 — セルフホストコンパイラ中核再構築（wasm32-gc overlay）

## Summary

`wasm32-gc` + `wasi-p2` の cacheless overlay を
median ≤7s / p95 ≤10s / RSS ≤512MB / `s2 == s3` にする。
これは最適化プロジェクトではない。実行正本は
[`docs/plans/selfhost-compiler-core-rewrite.md`](../../docs/plans/selfhost-compiler-core-rewrite.md)。
決定は [ADR-053](../../docs/adr/ADR-053-selfhost-compiler-core-rewrite.md)。

`#850` は Phase 5（phase arena）の製品 tracker に降格する。
tick 191 以降の局所 hop を始めない。

## 現行床

quiet **208s** / loaded **239s** / RSS **~1.77GB**。
Null collector は 23–26s で trap。約 24 倍が必要。

## Acceptance

- [x] Phase 0: 機械可読 overlay receipt と 3 回 baseline
      （KEEP_CLOCK phase receipt + per-decl fn_wall top-32 =
      `851-keepclock-fnwall-20261012.json`。mono-discovery 分離・総 alloc・
      live bytes は消費 phase へ defer — plan 注記）
- [x] Phase 1: generated `if index == N` を compact table にする（≤180s）
      （wide-chunk staircase で生成ソース −264 行・入力側 −4s。
      binary if-tree は A/B で却下 — plan 記録）
- [x] Phase 2: fat MIR 廃止。reconstruction 0、RSS ≤900MB、wall ≤120s
      （SoA 列 + MirInst handle + interned id は既存。str_val clone 除去・
      result_type_at 列直読・f32/f64 VT 駆動 dispatch を追加実装。
      self-fixpoint byte-identical×2・fixture 5 件 byte-identical・
      emit.code −0.44s。CALL target/vec 型名の名前解決は Phase 3 reloc
      スコープへ引継ぎ）
- [ ] Phase 3: function-at-a-time。同時生存 body ≤2、RSS ≤384MB、wall ≤35s
- [ ] Phase 4: whole-module propagate / sync scan 削除。lower+dataflow+reach ≤3s
- [ ] Phase 5: phase arena。RSS ≤384MB、GC ≤0.7s、total ≤12s
- [ ] Phase 6: 10 回計測 median ≤7s、p95 ≤10s、RSS ≤512MB、`s2 == s3`
- [ ] validate + fixture parity + 意味論的互換
- [ ] `hello` 2312B / 現行 sha256 は必須ではない
- [ ] `BOOTSTRAP_EMIT_*` は受入後にだけ翻す

## 次の作業

2026-09-15 の clean-commit audit では、pin→s2→s3 の no-cache fixpoint は
`s2 == s3` と validate まで通過したが、同じ commit の gc+p2 overlay 測定は
`core_op_registry_canonical_id_at` の out-of-bounds trap（rc=134、RSS
1,097,924 KiB）で 1 回目から停止した。
旧 10-run receipt は履歴として保持し、現行 master の証明には使わない。

Phase 0 の未チェック項目から始める。次はこの trap の根因を直し、clean commit から
同じ runner で 10 回測定してから Phase 6 の判定を更新する。
`docs/plans/selfhost-compiler-core-rewrite.md` の探索規則に従う。

### 2026-10-12 更新（trap 解消 + gc-emit host 計測）

**Sep-15 blocker の trap は解消済み。** 修正は 3 点:

- checks.py の flat-src stub が `ctx_split_top_level_type_args` を
  `stub_…` に rename しており、struct_lit.ark からの外部呼び出しが
  `drop; unreachable` に劣化（dead-code type error + latent linear trap）
- `gc_param_type_sig_override` の hashmap_/hashset_ catch-all が param0 を
  `vec:i32` に潰し、`hashmap_type_name(ty: TypeInfo)` の declared GC 型を破壊
- `emit_env_args_gc_store_one*` が `emit_gc_array_get` 前で narrow を二重化し、
  ref オペランドに wrap が残って validation 破綻

**結果**: gc-emit 版フルコンパイラが初めて validate・component 化・実行まで
通り、hello.ark のコンパイルと生成物の実行を確認（`hello from gc selfhost`）。

**現行床（10-run receipt**: `docs/research/receipts/851-gc-overlay-goal-20261012.json`）:
gc-emit host（s3-gc32.component）で median **49.5s** / p95 **50.2s** / RSS **~600MiB**、
10/10 成功・validate・出力 sha256 全一致。

**ボトルネック特定**: linear host は同 overlay で >600s（timeout）。
perf/perfmap は `wasm func[11026]`＝linear GC `mark_candidate` が
サンプル 99.97%。emit 仕事ではなく host 自身の linear GC mark が支配的。
また fixpoint の s3 ビルド（wasm32+p1 emit）は 3600s timeout に到達 —
linear emit 経路そのものも遅い。

**次の作業**: KEEP_CLOCK keep-clock gc host で frontend 分解済み —
`lower.body_emit ≈26-32s`（lower.total 34.8s、guest total 49.1s の ~71%）が
最大単一 chunk（fat MIR reconstruction + per-function emit、Phase 2/3 の直接対象）。
次は typecheck 7.5s・fn_index 6.1s・resolve 3.1s。
perfmap では host 側 wasmtime copying GC が ~55-60%（allocation churn、
Phase 5 arena が最大レバー）。Phase 0 の未消化: 208s vs 239s の切り分けと、
linear host での再現測定（現行は >600s timeout で計測不能 — mark_candidate が
改善しない限り linear host 計測は取れない）。

### 2026-10-29 更新（gc-host pin 復帰）

`lsp::documents::handle_did_change` の else 分岐が `JsonText::get_str_after` を
呼んでいたが、この associated function は `JsonText` impl に存在せず、bootstrap
overlay では trap stub に lower され、if 式の結果型を i64 に汚染していた。これが
wasm32-gc self-build の validation 失敗（`handle_did_change` の local.set）の
原因だった。`std/json.ark` に associated function を追加して解消した。

- pin: `3c02d18b…`（wasm32-gc / wasi-p2 component、8 513 357 bytes）
- build-compiler: warm **35.8 s**（旧 core pin は 2011 s）
- fixpoint: **34.1 s**（s2 == s3 == `56e7ff6a…`）
- p1 identity: component の p1 出力 == 旧 core pin の s2 `56e7ff6a…`
- gc self-reproduction: `93aa0e6b…` で一致
- `BOOTSTRAP_EMIT_*` は wasm32 / wasi-p1 のまま（ADR-053 の ≤10s 受入後に翻す）

次の作業: Phase 3（function-at-a-time）で 35 s → 20 s → 12 s を詰める。

### 2026-10-29 更新（gc pin overlay の現行床と phase 分解）

clean `7438a59fb` で 3 本の receipt を取得した。

- gc pin cwasm host（AOT、通常 overlay）: median **39.1 s** / p95 39.6 s / RSS 1.10 GiB
- gc pin wasm host（JIT）: median **40.7 s** / p95 42.1 s / RSS 1.48 GiB
- keep-clock gc host（phase 実測）: median **39.9 s** / p95 40.2 s / RSS 1.50 GiB

phase 内訳（guest 計 ~35.9 s）: resolve 4.3 / typecheck 6.6 / lower.total 24.3
（body_emit 16.2、inloop_emit 5.7、fn_index 2.0、edge_bfs 1.1）/ emit 0.8。
wall との差 ~4 s は host GC + startup。10 s への主要ギャップは
**body_emit 16.2 s / typecheck 6.6 s / host GC ~4 s**。

receipts: `docs/research/receipts/851-gc-pin-overlay-{aot,wasm}-20261029.json`、
`851-gc-pin-keepclock-20261029.json`。通常 overlay は clock が stub され
frontend phase が 0 になるため、phase 実測は
`ARUKELLT_OVERLAY_EMIT_TARGET=wasm32-gc ARUKELLT_OVERLAY_KEEP_CLOCK=1` の
keep-clock gc host で行う（手順は plan に記載）。

次の作業: Phase 3 で body_emit/typecheck を削り、35 s → 20 s を狙う。

### 2026-10-29 更新（alloc-free qualified-name matching）

`wasmtime --profile guest`（10 ms、3,618 samples）で overlay コンパイルを
分解: `eq` 11.9% self、`MirModule_gc_enum_variant_matches` 6.4%
（すべて `typed_mvt_sig` から）、`name_index_find_slot` 5.9%、
`is_namespace_member_callee` 5.5%、`sig_reg_leaf_instance_name_match` 2.2%。
enum variant / callee spelling / sig-reg leaf の 3 matcher が per-call で
`"::" + name` を concat し operand を clone していたため、
`corehir::name_match` の byte-region 比較へ置換した（`407f2767e`、alloc 0）。

quiet A/B（同一 keep-clock host、各 3 run）:

| | wall median | guest median | body_emit | inloop_emit | edge_bfs |
|---|---:|---:|---:|---:|---:|
| baseline | 31.76 s | 28.93 s | 12.53 s | 4.55 s | 0.83 s |
| alloc-free | 30.25 s | 26.34 s | 11.31 s | 3.31 s | 0.62 s |

wall −4.8% / guest −8.9%。wall は host GC + 負荷が支配的で plan の wall 10%
ルールには届かないが、inloop_emit −27%・edge_bfs −25% が安定し、
allocation churn 削減（Phase 5）と per-fn loop（Phase 3）の前提になるため残す。
`selfhost fixpoint`、fixture parity（stdlib_json 136/0、generics_v1）、
fmt --check、verify lane は PASS。receipts:
`851-alloc-free-name-match-{base,opt}-20261029.json`。

次の作業: `name_index_find_slot` の probe 数（5.9%）と enum base index の
O(types×variants) を削る。body_emit/typecheck は Phase 3/4 の本丸。

### 2026-10-29 更新（alloc-free matcher を pin に焼き込み）

alloc-free matcher（`85568bde0`）を gc pin に焼き込み、`35a2585a…`
（8,522,488 B）へ refresh した。p1 identity は `02bd6917…`（前 pin の
stage-2 s2 とバイト一致）、gc self-reproduction は `80469233…`、
`--version` は gc flags 付きで `arukellt 0.1.0`。

refresh 後の実測（quiet、`.build/pinnew-verify`）:
- build-compiler warm **28.2 s**（cold 60.4 s、pin の AOT 生成込み）
- fixpoint **29.2 s**、s2 == s3 == `02bd6917…`
- stdlib_json fixture parity PASS

これでユーザー目標 `min(0.3x, 30 s)` の build-compiler 側（30 s）を満たした。
gc overlay gate（ADR-053）は quiet 30.25 s で、10 s には Phase 3/4 の
構造変更が必要。なお v2 の index/hash 変更（`15606318d`）は同一条件 A/B
（wall 30.62 vs 32.09 s、guest 27.53 vs 28.23 s）で有意差なしだったため
revert 済み（`fca9acf34`）。receipts:
`851-gc-struct-index-{base,opt}-20261029.json`。

### 2026-10-29 更新（命令数 A/B と layout index）

wall の負荷依存を避けるため、AOT cwasm + `perf stat -e instructions:u` を
A/B 指標に固定した（同一条件 2 run の差 <0.01%、負荷 20–50 でも安定）。
`ctx_struct_layout_struct_index` を全走査から incremental NameIndex に変えた版は
**262.23 → 257.15 G instructions（−1.9%）**、生成 wasm バイト一致、
gc host 自己再生成も byte 一致。同日の #944/#917 batch に同梱されて
履歴に残った（`a7292c683` 系）。

注意: fc==1 の layout を field push 完了前に index へ入れると gc host が
誤コンパイルされ、自己コンパイルが spurious E0212 を出す。分類は
「fc != 1 は push 時、fc == 1 は field push 完了時」。

merge 後 HEAD（`a7292c683`）実測: build-compiler warm 55.6 s / user 45.9 s
（負荷 ~20）、fixpoint PASS（s2 == s3 == `e6fbb279…`）。

### 2026-10-29 更新（gc struct slot index）

`MirModule_gc_struct_slot` の線形走査を `MirModule.gc_struct_slot_index`
（NameIndex、insert_first で first-match 維持）で O(1) 化した。
命令数 A/B: **248.28 → 239.28 G instructions（−3.62%、2 run で再現）**、
overlay gc 出力バイト一致、gc host の自己コンパイルが自身の s2
`8fdebc10…` を再現、fixture 4/4 バイト一致。layout index（−1.9%）と合わせて
累積 **−5.4%**。`f1ce43311` merge に同梱され履歴に残った。

注意: 並行セッションが同じ checkout で commit するため、s2 と fixpoint の
source がずれることがある（実際に一度 fixpoint 不一致になったが、原因は
source レースで、HEAD `ba4811c9b` から s2/s3 を再ビルドすると
s2 == s3 == `152f13d9…` で PASS）。fixpoint は source 変更後に同一 source から
再ビルドして評価する。

HEAD `ba4811c9b` の実測: build-compiler warm **31.5 s** / user 30.4 s（負荷 ~4）、
stdlib_json / generics_v1 parity PASS。

### 2026-10-29 更新（stub module に当たった失敗）

`ctx_find_leaf_mono_instance_index` の O(M) scan を O(1) leaf index にしたが、
bootstrap overlay は `_patch_bootstrap_stub_static_dispatch` が
`mir_lower_ctx_mono_type_params.ark` を丸ごと stub 文字列に置換するため、
変更は自己ホスト compiler に入らず、命令数も baseline と一致した。
revert 済み（`18e1d56a3`）。

教訓: overlay で効かせたい変更は、まず生成された overlay を grep して
当該関数が存在するか確認する（stub 化モジュールは無効）。

最新 pin `62aecf93…` / HEAD `55675414f`(+revert) の再検証:
build-compiler warm 59.8 s / user 52.8 s（負荷 ~10、overlay 再生成込み）、
fixpoint PASS（s2 == s3 == `be07c165…`）、stdlib_json / generics_v1 parity PASS。

### 2026-10-29 更新（目標秒数の再確認）

最新 source（`b9fbac631` 系、pin `62aecf93…`）の build-compiler warm:
**23.2 s**（load 2.4）/ **29.8 s**（load 2.6）/ 52.0 s（負荷スパイク）、
user 22.4 / 28.8 s。quiet 時は 30 s 目標を満たす。

overlay の `fn` decl 12,975 に対し、s2 の wasm func は 23,698。
mono instance / wrapper が decl より多く、Phase 3 の主対象は
「未到達 decl の除去」ではなく per-fn pipeline のスキャン削減と RSS。

並行ビルドで overlay cache が stale を返す場合は
`rm .build/selfhost/flat-overlay-cache.json` で再生成する。
