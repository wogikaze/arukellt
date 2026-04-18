# T3: enum dispatch の br_on_cast 連鎖最適化

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-18
**Closed**: 2026-04-18
**ID**: 094
**Depends on**: —
**Track**: backend-opt
**Orchestration class**: design-ready
**Orchestration upstream**: Rescoped 2026-04-18 — original i31 / monolithic `br_on_cast` acceptance deferred; see §Rescope — 2026-04-18; partial IfStmt→`br_table` work landed under #505 (`issues/done/505-t3-br-table-type-info-gap.md`)
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/094-t3-br-on-cast-chain-opt.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

Arukellt の enum パターンマッチは T3 で `br_on_cast` / `br_on_cast_fail` の連鎖として emit されるが、
バリアントの出現頻度に基づいて最も高頻度のバリアントを先に試みる順序に並び替える
（プロファイルがない場合は、タグ番号の小さい順で最適化）。

また、連続する `br_on_cast` の対象型が完全に非交差の場合、
`br_table` による O(1) ディスパッチに変換できないかを検討する。

## 受け入れ条件

**Superseded (2026-04-18):** The three bullets below described an optimization
target that contradicts the current T3 GC enum representation (see §STOP_IF).
They are retained as historical scope; **close gate for this issue** is now
the documentation rescope in §Rescope — 2026-04-18, not a new emitter landing.

1. enum ディスパッチの `br_on_cast` 連鎖が3個以上の場合に最適化対象 *(historical)*
2. `br_table` への変換: i31 タグを使った O(1) ディスパッチ実装 *(blocked — requires repr change; deferred)*
3. パターンマッチを多用するベンチマークで実行時間改善を確認 *(out of scope until #505 / future repr work)*

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/MVP.md` §br_on_cast
- `docs/spec/spec-3.0.0/OVERVIEW.md` §GC詳細

---

## STOP_IF triggered — 2026-04-15

**Applied STOP_IF #2**: T3 does not currently emit `br_on_cast` chains for enum
*dispatch* (uses a different dispatch strategy entirely). Documenting actual
strategy here; implementation halted.

### Actual T3 enum dispatch strategy

T3 uses a **two-step** approach for enum pattern matching, not a monolithic
`br_on_cast` dispatch chain:

**Step 1 — Tag extraction (`Operand::EnumTag` in `operands.rs`)**

For enums with ≥ 3 variants at `opt_level >= 1`, `EnumTag(scrut)` is emitted
as a structured `BrOnCast` chain that:
1. Opens a `block (result i32)` and n inner typed ref blocks.
2. Emits the enum GC ref once onto the stack.
3. Chains `br_on_cast` instructions for each variant GC struct type.
4. On exit from each inner block: `Drop` the typed ref, push `I32Const(i)`,
   `Br` to the outer i32 result block.

This produces an `i32` discriminant — it is **not** O(1); it tests GC struct
types O(n) in the worst case.

For enums with < 3 variants or `opt_level == 0`, step 1 uses nested
`ref.test + if/else` chains instead (also O(n)).

**Step 2 — Arm dispatch (`MirStmt::IfStmt` chains in `stmts.rs`)**

The MIR pattern lowering (`crates/ark-mir/src/lower/pattern.rs`) compiles
`match e { V0 => arm0, V1 => arm1, V2 => arm2 }` to:

```
IfStmt { cond: BinOp(Eq, EnumTag(e), ConstI32(0)), then: arm0, else: [
  IfStmt { cond: BinOp(Eq, EnumTag(e), ConstI32(1)), then: arm1, else: [
    arm2_stmts   // last arm: no tag check
  ]}
]}
```

Each arm **re-evaluates** `EnumTag(e)` inline — there is no pre-cached `i32`
discriminant local. The T3 `stmts.rs` handler for `IfStmt` emits:

```wasm
; For each arm:
<EnumTag(e) via BrOnCast chain>  ; O(n) type tests
i32.const k
i32.eq
if (empty)
  <arm_k stmts>
else
  <remaining if/else chain>
end
```

**Overall complexity**: O(n) arms × O(n) BrOnCast tests per arm = **O(n²)**
type tests in worst case for a complete n-variant enum match.

### Why i31-tagged `br_table` is blocked (STOP_IF #1)

The acceptance criterion explicitly requires "i31-tagged enums". T3 represents
enum values exclusively as GC struct typed references — there is no i31
discriminant field stored in the enum objects. Adding one requires:

1. **Layout change**: Add an i31/i32 tag field to every enum GC struct type
   (affects `crates/ark-wasm/src/emit/t3_wasm_gc/types.rs`).
2. **EnumInit change**: Set the tag field on construction
   (`crates/ark-wasm/src/emit/t3_wasm_gc/stmts.rs` / `operands.rs`).
3. **EnumTag change**: Replace BrOnCast chain with a single `struct.get` of
   the tag field.
4. **EnumPayload change**: Field indices shift if tag field is prepended.
5. **Match dispatch change**: Unconditionally emit `br_table` on tag, not
   `if/else` chains.

Estimated change: ≥ 180 lines across 4 files — exceeds the 150-line limit
(STOP_IF #3 also applies independently).

### Feasible partial optimization (not blocked by STOP_IF)

Converting the **IfStmt chain** dispatcher to `br_table` — WITHOUT changing
the enum representation — is feasible within ~100 lines:

- Recognize "linear enum switch" pattern in `stmts.rs`:
  `IfStmt { cond: BinOp(Eq, EnumTag(same_scrut), ConstI32(k)), ... }`
- Emit `EnumTag(scrut)` once → scratch i32 local
- Use `br_table` on the scratch local to dispatch to arm blocks

This reduces step-2 from O(n) if/else to O(1) br_table dispatch. Step-1
(tag extraction) remains O(n) via BrOnCast, so overall is O(n) not O(n²).
This optimization was tracked at: `issues/done/505-t3-br-table-type-info-gap.md` (done).

### Summary

| Aspect | Current T3 | This issue asks for |
|---|---|---|
| Enum repr | GC struct types | i31-tagged GC structs |
| Tag extraction | O(n) BrOnCast chain | O(1) struct.get of tag field |
| Arm dispatch | O(n) if/else | O(1) br_table |
| Overall | O(n²) worst case | O(1) |
| Blocker | Repr change > 150 lines | — |

---

## Rescope — 2026-04-18 (design slice, `impl-compiler`)

**Purpose:** Lift the hard **STOP_IF** gate for orchestration by recording the
canonical “actual T3 behavior vs original acceptance” split in-repo, and
routing feasible emitter work to the correct tracker.

**Decisions**

1. **No dispatch** of this issue’s *original* i31-tagged `br_table` acceptance
   until enum representation carries an explicit integer tag (see §Why
   i31-tagged `br_table` is blocked).
2. **Partial dispatch** (IfStmt chain → `br_table` on a cached `EnumTag`
   discriminant, still O(n) tag extraction) was tracked under **#505**
   (`issues/done/505-t3-br-table-type-info-gap.md`), not under #094.
3. #094 remains a **design / audit anchor**: STOP_IF sections below document
   emitter reality (`crates/ark-mir/src/lower/pattern.rs`,
   `crates/ark-wasm/src/emit/t3_wasm_gc/stmts.rs`, `operands.rs`).

**New acceptance (documentation-only, this slice)**

- [x] Issue documents why original acceptance is incompatible with HEAD (§STOP_IF + table above).
- [x] Follow-up implementation path cites #505 (`issues/done/505-…`) for `br_table` dispatch without repr change.
- [x] Frontmatter `Status` / orchestration class updated so agents do not dispatch blind optimization.

**Verification**

- `bash scripts/run/verify-harness.sh --quick` → exit 0 (2026-04-18).

**STOP_IF historical note:** The §STOP_IF section remains as **audit evidence**;
it is no longer interpreted as “block all work on this issue” now that scope is
explicitly documentation + routing.
