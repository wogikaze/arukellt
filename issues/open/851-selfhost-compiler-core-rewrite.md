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

- [ ] Phase 0: 機械可読 overlay receipt と 3 回 baseline
- [ ] Phase 1: generated `if index == N` を compact table にする（≤180s）
- [ ] Phase 2: fat MIR 廃止。reconstruction 0、RSS ≤900MB、wall ≤120s
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

**次の作業**: frontend（parse〜lower）の時間分解。gc host の ~50s のうち
emit は ~2.6s で、残りは frontend だが現行ビルドは `--time` が stub され
0ms 表示。KEEP_CLOCK ビルドか perfmap で frontend の内訳を取り、
Phase 0 の receipt 項目を完成させる。
