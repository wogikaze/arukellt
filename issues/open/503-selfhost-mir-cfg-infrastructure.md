# Issue #503 — Selfhost MIR: CFG and Dominance-Frontier Infrastructure for SSA

## Status: OPEN

**Blocker for:** #494 (Selfhost MIR SSA formation pass — phi-node insertion)

---

## Problem

Issue #494 assigned an SSA formation pass (phi-node insertion at join points) to `src/compiler/mir.ark`. During analysis the following structural gaps were identified that block implementation:

### Gap 1 — CFG is not built during HIR→MIR lowering

`MirBlock` has `succ0` and `succ1` fields for successor blocks, but:

- The lowering creates exactly **one block per function** (`alloc_block` is called once at function entry).
- All if/else/while/loop/match control flow is encoded as **structured instruction markers** (`MIR_IF`, `MIR_ELSE`, `MIR_END`, `MIR_BLOCK`, `MIR_LOOP`, `MIR_BR_IF`) emitted sequentially into that single block.
- `succ0` and `succ1` are never populated during lowering.
- There are no predecessor lists.

As a result, there is no actual CFG (multi-block, edge-connected) at the point where an SSA pass could run.

### Gap 2 — No dominance-frontier data structures

SSA formation (Cytron et al. 1991) requires:

1. **Predecessor lists** per block  
2. **Immediate dominator** computation (e.g., Lengauer–Tarjan or iterative bit-vector)  
3. **Dominance frontier sets** per block  
4. A **phi-node instruction type** (`MIR_PHI`) with block-labelled arguments

None of these exist in `src/compiler/mir.ark` or elsewhere in the selfhost pipeline.

---

## Scope Required to Unblock #494

The following must be added before the SSA pass from #494 is feasible:

### A — Change if/else/while lowering to produce multiple blocks

Currently:
```
// single block 0
MIR_IF
  (then-body instructions)
MIR_ELSE
  (else-body instructions)
MIR_END
```

Required for SSA:
```
// block 0 (entry):  ...cond...  terminate with BR_IF(then_bb, else_bb)
// block 1 (then_bb): (then-body instructions)  terminate with BR(join_bb)
// block 2 (else_bb): (else-body instructions)  terminate with BR(join_bb)
// block 3 (join_bb): phi(%result = phi [%a, then_bb], [%b, else_bb])  ...
```

This requires changing `lower_expr` for `NK_IF_EXPR`, `NK_WHILE`, `NK_LOOP`, and `NK_FOR` to:
- call `alloc_block` for then/else/body/exit/join blocks
- emit a proper `MIR_BR_IF` terminator (branching to two successor block IDs) at the end of the header block
- set `block.succ0` / `block.succ1` correctly
- set `ctx.cur_block` to the new block after each transition

### B — Add predecessor lists to `MirBlock`

```ark
struct MirBlock {
    // existing fields ...
    preds: Vec<i32>,   // IDs of predecessor blocks
}
```

After all blocks are created, compute predecessors from `succ0`/`succ1`.

### C — Add dominance computation

```ark
struct DomInfo {
    idom: Vec<i32>,       // immediate dominator for each block (-1 = none)
    dom_frontier: Vec<Vec<i32>>,  // dominance frontier set per block
}
```

Implement iterative dominator computation (simple Cooper et al. dataflow) over the predecessor/successor graph.

### D — Add `MIR_PHI` instruction and phi-node type

```ark
fn MIR_PHI() -> i32 { 80 }

struct MirPhiArg {
    val_local: i32,
    pred_block: i32,
}

struct MirPhi {
    dest: i32,
    args: Vec<MirPhiArg>,
}
```

The `MirBlock` would need a `phis: Vec<MirPhi>` field, or phi instructions could be emitted at the block's start as special `MirInst` entries.

### E — Implement phi insertion pass

After CFG is built:
1. Find all variables defined in more than one block
2. Insert phi nodes at dominance frontiers of those blocks (iterated frontier)
3. Rename variables to SSA form (fresh name per definition point)

---

## Effort Estimate

This is a ~3–5 issue decomposition:

| Sub-issue | Scope |
|---|---|
| 503a | Change if/while/loop lowering to produce multi-block CFG |
| 503b | Add predecessor computation to MirBlock |
| 503c | Add dominance computation (idom + frontier) |
| 503d | Add MIR_PHI instruction and MirPhi struct |
| 503e | Implement phi-insertion and variable renaming (= #494 proper) |

---

## Note

The structured control-flow approach currently used (MIR_IF/ELSE/END) is appropriate for direct Wasm emission because Wasm uses structured control flow natively. However, that representation does not support traditional SSA phi nodes, which require an explicit CFG.

A design decision is needed: should the selfhost MIR pipeline move to an explicit CFG representation (which would then feed a Wasm code generator that re-structures the CFG into Wasm blocks), or should SSA be avoided entirely and the structured representation extended instead?

This decision should be captured as an ADR before #503a–e work begins.

---

## References

- Cytron et al. 1991, "Efficiently Computing Static Single Assignment Form and the Control Dependence Graph"
- Cooper et al. 2001, "A Simple, Fast Dominance Algorithm"
- `src/compiler/mir.ark` — current MIR implementation
- `issues/open/494-selfhost-mir-ssa-formation.md` — blocked issue
