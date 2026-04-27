---
Status: done
Created: 2026-04-15
Updated: 2026-04-15
ID: 505
Track: backend-opt
Depends on: —
Orchestration class: implementation-ready
---
# T3: br_table enum dispatch — feasible IfStmt-chain optimization
**Blocks v4 exit**: no

---

## Summary

This issue documents a **feasible** T3 optimization spun off from issue
`094-t3-br-on-cast-chain-opt.md`, which was halted by STOP_IF (i31-tagged
enum representation required > 150 lines of structural changes).

The optimization here does NOT require changing the enum representation.
It converts the O(n) `if/else`-chain arm dispatch into O(1) `br_table`
dispatch, while leaving the O(n) `BrOnCast` tag-extraction step unchanged.
Overall complexity improves from O(n²) to O(n).

## Background — actual T3 dispatch strategy

See `issues/open/094-t3-br-on-cast-chain-opt.md` §"Actual T3 enum dispatch
strategy" for the full analysis. Summary:

1. **Tag extraction** (`EnumTag` in `operands.rs`): BrOnCast chain, O(n).
2. **Arm dispatch** (`MirStmt::IfStmt` chain in `stmts.rs`): each arm
   re-evaluates `EnumTag` inline — O(n) if/else, re-extracts tag O(n) each
   time → O(n²) worst case for complete n-variant match.

## Proposed optimization

**In `crates/ark-wasm/src/emit/t3_wasm_gc/stmts.rs`**, when emitting
`MirStmt::IfStmt` at `opt_level >= 1`:

1. **Detect "linear enum switch" pattern** — a chain of:

   ```
   IfStmt { cond: BinOp(Eq, EnumTag(Place(local_x)), ConstI32(k_0)), ... }
   else [IfStmt { cond: BinOp(Eq, EnumTag(Place(local_x)), ConstI32(k_1)), ... }]
   else [... last arm has no EnumTag condition]
   ```

   where all `EnumTag` args refer to the **same local** `local_x` and n ≥ 3.

2. **Emit tag once**: `EnumTag(Place(local_x))` → `si(9)` (or a dedicated
   scratch i32 local added as `si(13)`).

3. **Emit `br_table` dispatch**:
   - Open n + 1 nested empty Wasm blocks (`$done` outer + n arm blocks).
   - `local.get $tag_scratch`
   - `br_table [0..n-2] default=n-1` (arms sorted by tag value; last arm=default)
   - For each arm body: `End`, emit body stmts, `Br(n-1-i)` to exit `$done`.
   - Final `End` closes `$done`.

4. **Fallback** (no pattern match): emit the existing if/else chain unchanged.

## Acceptance criteria

- [x] `IfStmt` chains of ≥ 3 arms where all conditions are
  `BinOp(Eq, EnumTag(same_local), ConstI32(k))` are compiled to `br_table`
  at `opt_level >= 1`.
- [x] Arms NOT matching this pattern (guards, non-EnumTag conds, or < 3 arms)
  use the existing if/else path — no regression.
- [x] A test fixture (WAT or test function) confirms `br_table` appears in the
  emitted WASM for a 3+ variant enum match.
- [x] `cargo test` passes.
- [x] `bash scripts/run/verify-harness.sh --quick` passes.

## Completion note — 2026-04-15

Resolved by commit `6df3013`. Linear enum-switch `IfStmt` chains now lower to
`br_table` in the T3 backend at optimized levels, with fallback to the legacy
if/else path for non-matching shapes and regression coverage for emitted Wasm.

## Implementation scope estimate

| Component | Change | Lines |
|---|---|---|
| `stmts.rs` — pattern recognizer | new helper fn | ~35 |
| `stmts.rs` — br_table emitter | new helper fn | ~55 |
| `stmts.rs` — IfStmt entry point | call helpers | ~8 |
| `helpers.rs` — si(13) i32 scratch | 1 push | ~1 |
| test fixture | new .ark + expected | ~20 |
| **Total** | | **~119** |

Well within the 150-line limit.

## Notes

- The pattern recognizer must handle the case where the last arm has an empty
  `else_body` (no final else) vs. a wildcard/catch-all last arm.
- Arms may appear in any tag order in the source; `br_table` requires
  sorting by tag value for the dispatch table.
- Guards in conditions (`BinOp(And, BinOp(Eq, EnumTag(...), ...), guard)`)
  break the pattern — fall back to if/else for such arms.
- The BrOnCast tag-extraction step (step 1) remains unchanged. Only step 2
  (arm dispatch) is optimized.

## References

- `issues/open/094-t3-br-on-cast-chain-opt.md` — parent issue (halted)
- `crates/ark-wasm/src/emit/t3_wasm_gc/stmts.rs` — IfStmt emission
- `crates/ark-wasm/src/emit/t3_wasm_gc/operands.rs` — EnumTag emission
- `crates/ark-wasm/src/emit/t3_wasm_gc/cabi_adapters.rs` — br_table pattern
  reference (see `emit_i32_to_enum_ref` for the Wasm structure to adapt)