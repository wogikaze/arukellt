---
Status: done
Created: 2026-03-31
Updated: 2026-04-14
ID: 283
Track: corehir
Depends on: 281
Orchestration class: implementation-ready
Orchestration upstream: —
---

# CoreHIR lowering: TryExpr を制御フローに desugar する
**Blocks v1 exit**: no
**Priority**: 3

## Reopened by audit — 2026-04-13

**Reason**: CoreHIR TryExpr still backend-illegal.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

`Operand::TryExpr` (`expr?` 構文) が backend-illegal のまま。match-on-result + early-return への変換が必要。

## Current state

- `crates/ark-mir/src/mir.rs`: `TryExpr { expr, from_fn }` が Operand enum に存在
- legacy path では `?` を result 分岐に展開している
- CoreHIR path では未処理

## Acceptance

- [x] `Operand::TryExpr` が match-on-Result + early-return 形式の MirStmt 列に変換される
- [x] `?` 演算子を含む fixture が CoreHIR path 単独で compile & run 成功
- [x] `validate_backend_legal_module` が try 含む MIR で pass する

## Progress

- 2026-04-18: Added `lower_try_expr_removes_tryexpr_from_return_terminator` in `crates/ark-mir/src/lower/tests.rs` to cover `Terminator::Return(Some(TryExpr))` desugar (distinct from assign+rvalue path).

## References

- `crates/ark-mir/src/lower/mod.rs`
- `crates/ark-mir/src/lower/func.rs`
- `crates/ark-mir/src/mir.rs`

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
