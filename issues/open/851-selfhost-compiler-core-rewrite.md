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
