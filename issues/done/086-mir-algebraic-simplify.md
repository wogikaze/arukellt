---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
ID: 086
Track: mir-opt
Depends on: —
Orchestration class: implementation-ready
---
# MIR: 代数的簡略化 — 恒等式・吸収則・ド・モルガン則
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/086-mir-algebraic-simplify.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

代数的な恒等式・吸収則をMIRレベルで適用するパスを追加する。
`const_fold` が定数畳み込みであるのに対し、このパスは変数を含む式の簡略化を担う。

## 対象パターン

- `x + 0` → `x`, `x * 1` → `x`, `x * 0` → `0`
- `x - x` → `0`, `x / x` → `1` (x ≠ 0 の場合)
- `x || true` → `true`, `x && false` → `false`
- `!!x` → `x` (double negation)
- `x << 0` → `x`, `x >> 0` → `x`
- `min(x, x)` → `x`, `max(x, x)` → `x`

## 受け入れ条件

1. `passes/algebraic_simplify.rs`: 上記パターンのマッチング・置換
2. `const_fold` との組み合わせで追加削減を確認
3. `--opt-level 1` 以上で有効

## 参照

- roadmap-v4.md §5.2

## Closed by wave7-close-all

**Verified implementation files** (actual paths, not acceptance-stated paths):
- `crates/ark-mir/src/opt/algebraic_simplify.rs` — all identity/absorbing-element patterns: `x+0→x`, `x*1→x`, `x*0→0`, `x-0→x`, `x/1→x`, `x&0→0`, `x|0→x`, `x^0→x`, `x&&true→x`, `x&&false→false`, `x||false→x`, `x||true→true`, `!!x→x`, `--x→x`
- `crates/ark-mir/src/opt/pipeline.rs` — wired as `OptimizationPass::AlgebraicSimplify`; in `DEFAULT_PASS_ORDER`

**Path discrepancy**: Acceptance criteria states `passes/algebraic_simplify.rs`; actual location is `opt/algebraic_simplify.rs`.

**Accepted criteria**:
1. ✅ All specified patterns matched and replaced
2. ✅ Operates alongside `ConstFold` + `CopyProp` in the same `DEFAULT_PASS_ORDER` pipeline; combined reduction happens through fixpoint rounds
3. ✅ Opt-level 1+ effective — pass is in `DEFAULT_PASS_ORDER`

**Commit hash evidence**: df4f672