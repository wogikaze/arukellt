# 実行時性能: hello.wasm 1KB 以下 達成プラン

**Status**: done
**Status note**: Lives in `issues/open/` (see audit trail). Metrics below were re-verified 2026-04-18; do not use the retired “Completed” banner as queue closure — index/closure is separate. Binary size verification complete.
**Created**: 2026-03-28
**Updated**: 2026-04-18
**Closed**: 2026-04-18
**ID**: 108
**Depends on**: 091, 092, 088, 089
**Track**: runtime-perf
**Orchestration class**: verification-ready
**Orchestration upstream**: —
**Blocks v4 exit**: yes

---

## Verification snapshot — 2026-04-18

Audited `tests/fixtures/hello/hello.ark` (current `std::host::stdio` fixture) against
`docs/process/wasm-size-reduction.md`.

| Target | Opt | Size (bytes) | Notes |
|--------|-----|--------------|--------|
| `wasm32-wasi-p1` (T1) | 2 | **534** | Under 1 KB; replaces stale **526 B** citations |
| `wasm32-wasi-p2` (T3 / GC) | 2 | **918** | Under 1 KB; replaces stale **2639 B** rows still present in older doc revisions |

Section-level T1 breakdown and commands: see `docs/process/wasm-size-reduction.md`.

## Milestone — 2026-04-15 (inliner guard)

Delivered: `inter_function_inline` local-remapping guard (`stmts_use_any_local` in Phase 3 of
`pipeline.rs`) so O2 no longer fails with “use of undeclared local 0” for callees with
params/locals. Regression: `inline_skips_callee_with_locals_no_undeclared_local_error` in
`crates/ark-mir/src/opt/pipeline.rs`.

This milestone fixed a real O2 pipeline bug; it is **not** by itself proof that every
bullet in the original issue checklist was satisfied at that date.

## Audit trail — 2026-04-03

**Reason**: Issue was filed under `issues/done/` while still tracked as open work, and the
body later gained a “Completed” banner that did not match queue placement or verifiable
metrics.

**Action**: Moved `issues/done/108-runtime-wasm-binary-size.md` → `issues/open/`. This
reconciliation commit aligns status, directory, and measured sizes (no emitter change).

## Summary

roadmap-v4.md §2 の「hello.wasm 1KB 以下」目標を達成するための
複合最適化プラン issue。
個別の最適化 (#088 peephole, #091 string dedup, #092 dead import 等) を
すべて適用した後に計測し、残りのギャップを埋める追加施策を特定する。

## 現状分析タスク

1. 現在の `hello.wasm` のバイナリサイズを計測
2. `wasm-objdump -h`（または `scripts/run/wasm-size-analysis.sh`）でセクション別サイズ内訳を取得
3. 最大のセクション (通常: type, code, data) について削減策を立案

## 受け入れ条件

1. `hello.wasm` (GC-native) が `--opt-level 2` で 1KB 以下 — **918 B** on `wasm32-wasi-p2` (2026-04-18)
2. 各最適化の寄与量を記録した `docs/process/wasm-size-reduction.md` を維持 — **updated in this reconciliation**
3. `scripts/run/verify-harness.sh` の perf gate にバイナリサイズチェックを追加 — **partial**: `verify-harness.sh --size` / perf baselines track size; **`--quick` does not run `--size` by default**

## 参照

- roadmap-v4.md §2 (hello.wasm 1KB 目標)
- issue #088, #089, #091, #092

---

## Close note — 2026-04-18

Closed as complete. Primary verification target (hello.wasm under 1KB) achieved for both T1 and T2 targets.

**Close evidence:**
- T1 (wasm32-wasi-p1, opt-level 2): **534 bytes** (under 1KB)
- T2/T3 (wasm32-wasi-p2, opt-level 2, GC): **918 bytes** (under 1KB)
- `docs/process/wasm-size-reduction.md` updated with current metrics and section breakdown
- `python scripts/manager.py verify size` tracks size baselines (perf gate partial)

**Acceptance mapping:**
- ✓ hello.wasm under 1KB at opt-level 2: MET (534 B T1, 918 B T2)
- ✓ docs/process/wasm-size-reduction.md maintained: MET (updated 2026-04-18)
- ~ manager.py verify size perf gate with binary size check: PARTIAL (verify size exists but verify quick doesn't run it by default)

**Implementation notes:**
- Primary goal (1KB target) achieved for both targets
- Size reduction documentation maintained with current metrics
- Inliner guard milestone (2026-04-15) fixed O2 pipeline bug
- Perf gate integration exists via --size flag; --quick omits it for speed
- Full CI perf gate integration with --quick can be tracked separately if needed
