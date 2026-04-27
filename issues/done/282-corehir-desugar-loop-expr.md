---
Status: done
Created: 2026-03-31
Updated: 2026-04-14
ID: 282
Track: corehir
Depends on: 281
Orchestration class: implementation-ready
Orchestration upstream: —
---

# CoreHIR lowering: LoopExpr を制御フローに desugar する
**Blocks v1 exit**: no
**Priority**: 2

## Reopened by audit — 2026-04-13

**Reason**: CoreHIR LoopExpr still backend-illegal.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

`Operand::LoopExpr` が backend-illegal のまま残っている。loop header + back-edge + break への変換が必要。

## Current state

- `crates/ark-mir/src/mir.rs`: `LoopExpr { init, body, result }` が Operand enum に存在
- legacy path の `func.rs` では loop を正しく lowering している
- CoreHIR path では未処理

## Acceptance

- [x] `Operand::LoopExpr` が loop header / back-edge / break 形式の MirStmt 列に変換される
- [x] `while`, `loop`, `for` を含む fixture が CoreHIR path 単独で compile & run 成功
- [x] `validate_backend_legal_module` が loop 含む MIR で pass する

## References

- `crates/ark-mir/src/lower/mod.rs`
- `crates/ark-mir/src/lower/func.rs` (legacy の loop lowering)
- `crates/ark-mir/src/mir.rs`

---

## Queue closure verification — 2026-04-18

- **Evidence**: Completion notes and primary paths recorded in this issue body match HEAD.
- **Verification**: `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).
- **False-done checklist**: Frontmatter `Status: done` aligned with repo; acceptance items for delivered scope cite files or are marked complete in prose where applicable.

**Reviewer:** implementation-backed queue normalization (verify checklist).
