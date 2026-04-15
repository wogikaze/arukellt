# ADR-024: Selfhost MIR uses an explicit CFG before SSA formation

**Status**: DECIDED  
**Created**: 2026-04-15  
**Scope**: selfhost MIR, SSA formation, lowering, codegen boundary

## Context

Issue #494 assigns SSA formation for the selfhost MIR pipeline, including
phi-node insertion at join points. The current MIR implementation does not yet
provide the graph structure that SSA requires:

- `src/compiler/mir.ark` defines `MirBlock` with `succ0` / `succ1` fields, but
  HIR→MIR lowering never populates them.
- `lower_expr` emits structured markers such as `MIR_IF`, `MIR_ELSE`,
  `MIR_END`, `MIR_BLOCK`, `MIR_LOOP`, and `MIR_BR_IF` into a single lowered
  block per function.
- `issues/open/503-selfhost-mir-cfg-infrastructure.md` already documents the
  missing predecessor lists, immediate dominators, dominance frontiers, and
  phi-node support as the blocker for #494.

That means the current MIR is not just missing a pass; it lacks the explicit CFG
representation that a standard SSA algorithm consumes.

## Decision

Selfhost MIR must move to an explicit control-flow graph before SSA formation.

The canonical MIR representation for the selfhost pipeline will therefore be:

- multiple basic blocks per function
- explicit terminator edges for conditional and unconditional branches
- populated successor and predecessor lists
- block-level dominance information
- dominance-frontier sets available to the SSA pass
- a phi-node representation that is attached to join blocks

Structured control-flow markers may still exist as a temporary lowering aid or
as a backend-specific re-structuring step, but they are not the canonical
representation that the SSA pass should consume.

## Why this is required before #494

SSA formation needs graph information that the current structured-only lowering
does not provide:

1. Phi insertion requires predecessor lists for each join block.
2. Dominator and dominance-frontier computation require a real CFG, not a single
   instruction stream with nested `IF` / `ELSE` / `END` markers.
3. Variable renaming for SSA must be anchored to explicit block boundaries and
   join points.

If #494 tried to build SSA directly on the current structured MIR, it would
first need to reconstruct a CFG from markers, then compute dominance, then
insert phi nodes. That duplicates the same graph work inside the SSA pass and
keeps the representation ambiguous for later analyses.

## Rationale

1. **Matches the algorithmic contract**: the standard SSA algorithm assumes a
   CFG with predecessors, dominators, and dominance frontiers.
2. **Removes representation ambiguity**: a block graph is easier to reason
   about than nested structured markers when joins and backedges matter.
3. **Keeps later passes honest**: once the MIR is explicit CFG, every analysis
   pass sees the same control-flow structure that SSA uses.
4. **Preserves backend flexibility**: Wasm codegen can still re-structure the
   CFG into Wasm blocks/loops/ifs at emission time, but that is a codegen
   concern rather than the MIR contract.

## Consequences

- #494 remains blocked until #503 adds CFG construction, predecessor
  computation, dominator data, dominance-frontier data, and phi support.
- `src/compiler/mir.ark` must treat explicit block graph construction as part of
  the selfhost MIR lowering contract, not as an optional optimization.
- The Wasm backend remains free to emit structured Wasm from the CFG, but it
  should no longer rely on the structured MIR markers as the primary
  representation for SSA-related work.

## Alternatives considered

### A. Keep structured MIR as the primary representation and infer CFG only in
the SSA pass

Rejected. This pushes the same graph reconstruction problem into #494 and
duplicates control-flow logic in the wrong layer.

### B. Extend structured MIR with ad hoc phi handling

Rejected. Phi nodes still require explicit predecessor-aware join points, so the
representation would become a CFG in practice without naming it as such.

### C. Avoid SSA entirely and keep structured MIR forever

Rejected. #494 explicitly depends on SSA formation, and the current codebase
already calls out dominance-frontier infrastructure as the missing prerequisite.

## References

- `src/compiler/mir.ark`
- `issues/open/503-selfhost-mir-cfg-infrastructure.md`
- `issues/open/494-selfhost-mir-ssa-formation.md`
