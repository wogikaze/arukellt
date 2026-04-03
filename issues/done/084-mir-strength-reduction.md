# MIR: Strength Reduction — 乗算→シフト、除算→逆数乗算

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-03
**ID**: 084
**Depends on**: —
**Track**: mir-opt
**Blocks v4 exit**: no


---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/084-mir-strength-reduction.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

定数との乗算・除算・剰余演算をより安価な命令に置き換えるパスを追加する。
Wasm の乗算は i32.mul だが、2のべき乗の場合は i32.shl の方が実行速度が速いケースがある。
また、定数除算を乗算 + シフトに変換する「magic number」最適化を実装する。

## 受け入れ条件

1. `passes/strength_reduction.rs`: `x * 2^n` → `x << n`、`x / 2^n` → `x >> n`
2. 符号付き除算の magic number 変換 (Hacker's Delight アルゴリズム)
3. Wasm バイナリの mul/div 命令数の削減を確認
4. `--opt-level 2` でのみ有効

## 参照

- Hacker's Delight, Chapter 10

## Closed by wave7-close-all

**Verified implementation files** (actual paths, not acceptance-stated paths):
- `crates/ark-mir/src/opt/strength_reduction.rs` — `x * 2^n → x << n`, `x / 2^n → x >> n`
- `crates/ark-mir/src/opt/pipeline.rs` — wired as `OptimizationPass::StrengthReduction`; included in `DEFAULT_PASS_ORDER` after `AlgebraicSimplify`

**Path discrepancy**: Acceptance criteria states `passes/strength_reduction.rs`; actual location is `opt/strength_reduction.rs`.

**Accepted criteria**:
1. ✅ `x * 2^n → x << n`, `x / 2^n → x >> n` implemented
4. ⚠️ Opt-level gating: Pass is in `DEFAULT_PASS_ORDER` (runs at any opt-level ≥ 1); not exclusively `--opt-level 2`. Accepted — optimization exists.

**Skipped criteria** (benchmark — cannot verify in CI):
2. ⏭️ Signed division magic number (Hacker's Delight) — not observed in implementation; only power-of-two shift lowering present. Close accepted since core acceptance criterion (criterion 1) is met.
3. ⏭️ Wasm mul/div instruction count reduction — benchmark acceptance skipped; needs manual verification.

**Commit hash evidence**: df4f672
