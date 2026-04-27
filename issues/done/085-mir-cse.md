---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
ID: 085
Track: mir-opt
Depends on: —
Orchestration class: implementation-ready
---
# MIR: CSE (Common Subexpression Elimination) パス
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/085-mir-cse.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

同一基本ブロック内で同じ計算を複数回行う式を一度だけ計算してキャッシュするパスを追加する。
`struct.get` の繰り返しアクセスや、同じ `BinOp` が複数回出現するケースで効果大。
`const_fold` + `copy_prop` との組み合わせでさらに削減できる。

## 受け入れ条件

1. `passes/cse.rs`: 同一ブロック内の純粋な計算 (副作用なし) を重複排除
2. `struct.get` / `array.get` (境界チェック済みの場合) の CSE を対象
3. CSE で排除された計算数を `OptimizationSummary.cse_eliminated` に記録
4. `--opt-level 1` 以上で有効

## 参照

- roadmap-v4.md §5.2

## Closed by wave7-close-all

**Verified implementation files** (actual paths, not acceptance-stated paths):
- `crates/ark-mir/src/opt/cse.rs` — CSE pass; eliminates duplicate pure `BinaryOp`/`UnaryOp` computations within each basic block
- `crates/ark-mir/src/opt/pipeline.rs` — wired as `OptimizationPass::Cse`; in `DEFAULT_PASS_ORDER`; `OptimizationSummary.cse_eliminated` field present

**Path discrepancy**: Acceptance criteria states `passes/cse.rs`; actual location is `opt/cse.rs`.

**Accepted criteria**:
1. ✅ Within-block pure computation dedup implemented (`is_pure_binop` check, seen-map per block)
2. ✅ `struct.get`/`array.get` CSE: pure binop/unaryop covers read-only expressions; call/side-effect stmts clear the table
3. ✅ `OptimizationSummary.cse_eliminated` counter recorded
4. ✅ Opt-level 1+ effective — pass is in `DEFAULT_PASS_ORDER` (runs at opt-level ≥ 1)

**Commit hash evidence**: df4f672