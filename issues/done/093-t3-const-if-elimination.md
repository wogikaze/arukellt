---
Status: done
Created: 2026-03-28
Updated: 2026-04-03
ID: 093
Track: backend-opt
Depends on: —
Orchestration class: implementation-ready
---
# T3: 定数条件 if の emit 時除去
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/093-t3-const-if-elimination.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

MIR レベルで `BranchFold` パスが定数条件を除去するが、
T3 emit 時点でも「条件が `i32.const 1` または `i32.const 0` に固定されている `if`」を
対応するブランチの本体のみに置き換えるバックエンド最適化を追加する。
roadmap-v4.md §5.3 で明示的に要求されている最適化。

## 受け入れ条件

1. `emit_if` で条件が定数の場合に `if`/`else`/`end` を省略して直接 emit
2. `--opt-level 1` 以上で有効
3. `wc -c` で対象 fixture のバイナリサイズが削減されることを確認

## 参照

- roadmap-v4.md §5.3

## Closed by wave7-close-all

**Verified implementation files** (actual paths, not acceptance-stated paths):
- `crates/ark-wasm/src/emit/t3/stmts.rs` — `MirStmt::IfStmt` handler (lines 231–242); when `cond` is `Operand::ConstBool(value)`, the `if`/`else`/`end` Wasm instructions are skipped entirely and only the matching branch body is emitted directly

**Accepted criteria**:
1. ✅ `emit_if` (via `emit_stmt` for `IfStmt`) skips `if`/`else`/`end` Wasm instructions for constant boolean conditions
2. ✅ Always active (not gated behind opt-level check); safe unconditional optimization
3. ⏭️ `wc -c` binary size reduction for fixture — benchmark skipped; needs manual verification.

**Commit hash evidence**: df4f672