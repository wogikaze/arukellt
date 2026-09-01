---
Status: open
Created: 2026-09-01
Updated: 2026-09-01
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

Phase 0 の未チェック項目から始める。
`docs/plans/selfhost-compiler-core-rewrite.md` の探索規則に従う。
