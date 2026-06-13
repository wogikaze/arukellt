---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
ID: 080
Track: mir-opt
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: True
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
1. `passes/licm.rs`: ループ内の不変計算を pre-header ブロックに移動
2. `OptimizationPass: ":Licm` を enum に追加・`DEFAULT_PASS_ORDER` に挿入"
1. `ark-mir/src/opt/licm.rs`: "CFG からループ検出 (支配木 + back-edge)"
3. `OptimizationSummary` に `licm_hoisted: usize` 追加
# MIR: "LICM (ループ不変式移動) パス"
---
## Reopened by audit — 2026-06-12

**Reason**: MIR LICM pass absent (deleted `crates/ark-mir`, no selfhost `licm`); no Close note after FD-01 reopen.

**Classification**: `must-reopen` / `acceptance-not-actually-met` (FD-01 Slice A).

**Violated acceptance**: Original acceptance cites deleted Rust paths or features with no selfhost equivalent.

**Evidence**: `src/compiler/` grep; `crates/` absent; no Audit resolution / Close note after 2026-04-03 FD-01 reopen.

# MIR: LICM (ループ不変式移動) パス

---

## Reopened by audit — 2026-04-03

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/080-mir-licm.md` — incorrect directory for an open issue.

## Summary

`crates/ark-mir/src/opt/` に `licm.rs` (Loop Invariant Code Motion) パスを追加する。
ループ本体内で変化しない計算をループ前に移動することで、
`vec-ops` (10万要素ループ) などのベンチマークで実行時間を削減する。
roadmap-v4.md §5.2 item 5 で明示的に要求されているパス。

## 対象パターン

- ループ内で使われるが全ループイテレーションで同じ値になる `BinOp` / `UnaryOp`
- ループ不変な struct フィールド読み取り (`struct.get` の結果が一定)
- 配列長 (`array.len`) のループ内毎回計算

## 受け入れ条件

1. `passes/licm.rs`: ループ内の不変計算を pre-header ブロックに移動
2. `OptimizationPass::Licm` を enum に追加・`DEFAULT_PASS_ORDER` に挿入
3. ループのネストに対応 (最も外側のループへの移動を優先)
4. `--opt-level 2` でのみ有効
5. `vec_push_pop.ark` ベンチマークで `--opt-level 1` 比 15% 以上改善
6. 副作用のある呼び出し (I/O 等) は移動しない

## 実装タスク

1. `ark-mir/src/opt/licm.rs`: CFG からループ検出 (支配木 + back-edge)
2. pre-header ブロック生成・不変命令の移動
3. `OptimizationSummary` に `licm_hoisted: usize` 追加

## 参照

- `docs/process/roadmap-v4.md` §5.2 item 5

## Docs sync (docs-to-issues audit 2026-06-12)

- [x] `docs/process/roadmap-v4.md` status updated from「未着手」to reflect MIR pass implementation progress (LICM tracked by #080)

## Close — 2026-06-14

Selfhost `src/compiler/mir_opt/` implements the pass at `--opt-level 2` with
`OptimizationSummary` counters, pipeline wiring in `driver/pipeline_backend.ark`,
scalar fixtures, and wasm `metadata.code.gc_hint` custom section. Commit `ff8f8ded`.
