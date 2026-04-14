# MIR Optimization Passes

This directory contains the canonical pass modules for the MIR optimization
pipeline.  Each module exposes:

```rust
pub const MIN_LEVEL: OptLevel;
pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats;
```

`run` is a no-op (returns `PassStats::default()`) when the requested level is
below `MIN_LEVEL`, so callers can always invoke every pass unconditionally and
let gating happen at the pass boundary.

---

## Pass catalogue

### Implemented in this directory

| Module | `MIN_LEVEL` | Description | Depends on |
|--------|-------------|-------------|------------|
| `const_fold` | O1 | Fold constant binary operations (`1 + 2` → `3`) at compile time. Reduces instruction count for downstream passes. | — |
| `dead_block_elim` | O1 | Remove basic blocks that are unreachable from the function entry (CFG trimming). Shrinks code and enables further DCE. | — |

### Remaining passes (still managed by `opt/pipeline.rs`)

| Pass name | `MIN_LEVEL` | Description | Depends on |
|-----------|-------------|-------------|------------|
| `branch_fold` | O1 | Replace `if(true/false)` with unconditional goto. | — |
| `cfg_simplify` | O1 | Collapse empty goto-only blocks. | `branch_fold` |
| `copy_prop` | O1 | Replace `x = y` chains with the original source. | — |
| `const_prop` | O1 | Substitute known-constant locals into later operands. | `const_fold` |
| `dead_local_elim` | O1 | Remove locals that are written but never read. | `copy_prop` |
| `unreachable_cleanup` | O1 | Strip instructions after an `unreachable` terminator. | `dead_block_elim` |
| `cse` | O1 | Common sub-expression elimination. | — |
| `inline_small_leaf` | O2 | Inline leaf functions with ≤ 8 statements and no calls. | `dead_local_elim` |
| `string_concat_opt` | O2 | Normalize adjacent string-concat patterns. | — |
| `aggregate_simplify` | O2 | Collapse single-element aggregates to plain use. | — |
| `algebraic_simplify` | O2 | Identity / absorbing element elimination (`x * 1` → `x`). | `const_fold` |
| `strength_reduction` | O2 | Replace mul/div by power-of-2 with shifts. | `const_fold` |
| `loop_unroll` | O2 | Unroll small constant-bound loops. | `const_prop`, `branch_fold` |
| `licm` | O2 | Loop-invariant code motion. | `const_prop` |
| `bounds_check_elim` | O2 | Remove provably-safe array bounds checks. | `const_prop` |
| `escape_analysis` | O2 | Scalar replacement for stack-allocated values. | — |
| `type_narrowing` | O2 | Narrow union/enum types based on branch conditions. | `branch_fold` |
| `gc_hint` | O3 | Annotate GC allocation sites for the runtime. | `escape_analysis` |
| `branch_hint_infer` | O3 | Infer `likely`/`unlikely` hints from profile data. | — |

---

## Opt-level gating reference

| `OptLevel` | Passes active |
|------------|---------------|
| `None` | (none — debug or migration builds) |
| `O1` | const_fold, branch_fold, cfg_simplify, copy_prop, const_prop, dead_local_elim, dead_block_elim, unreachable_cleanup, cse |
| `O2` | All O1 + inline_small_leaf, string_concat_opt, aggregate_simplify, algebraic_simplify, strength_reduction, loop_unroll, licm, bounds_check_elim, escape_analysis, type_narrowing |
| `O3` | All O2 + gc_hint, branch_hint_infer |

Passes whose `MIN_LEVEL` exceeds the requested level are silent no-ops — they
return `PassStats { changed: 0 }` without touching the module.

---

## Disabling individual passes

The `--no-pass=<name>` CLI flag (or `Session::disabled_passes`) suppresses a
named pass in the batch pipeline.  The `passes/` modules honour this at the
pipeline level; their `run` function itself is always safe to call.

---

## Adding a new pass

1. Create `passes/<pass_name>.rs` with the two required items:
   ```rust
   pub const MIN_LEVEL: OptLevel = OptLevel::O1; // or O2/O3
   pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats { … }
   ```
2. Declare `pub mod <pass_name>;` in `passes/mod.rs` and add to `run_all`.
3. Add a row to this README (module, level, description, dependencies).
4. If the pass also belongs in the batch pipeline, add the variant to
   `OptimizationPass` in `opt/pipeline.rs` and wire it in `run_pass`.

---

## T3 safety classification (wasm32-wasi-p2)

Updated: 2026-04-15 (issue #486).  The blanket T3 `O0` MIR override in
`crates/ark-driver/src/session.rs` was replaced with per-pass gating.

### GC-safe: enabled for T3 at O1

All O1 passes are GC-safe — they operate on IR structure or pure arithmetic
and never touch GC reference types (`anyref`, `eqref`, struct types, i31ref):

| Pass | Reason safe |
|------|-------------|
| `const_fold` | Pure constant arithmetic; no GC refs |
| `branch_fold` | Structural CFG replacement; no GC refs |
| `cfg_simplify` | Collapses empty goto blocks; structural only |
| `copy_prop` | Aliases scalar locals; T3 GC refs have distinct types |
| `const_prop` | Substitutes constants only |
| `dead_local_elim` | Removes write-only locals; GC refs are always read |
| `dead_block_elim` | CFG trimming; structural only |
| `unreachable_cleanup` | Strips after unreachable; structural only |
| `cse` | Deduplicates pure `BinaryOp`/`UnaryOp`; clears on all calls |

Safe O2 arithmetic passes also enabled for T3:

| Pass | Reason safe |
|------|-------------|
| `algebraic_simplify` | Pure arithmetic identities (`x * 1 → x`); no GC refs |
| `strength_reduction` | Power-of-2 shift substitution; no GC refs |
| `string_concat_opt` | String-specific; no GC ref involvement |

### GC-gated: disabled for T3 (O2/O3 passes only)

These passes remain disabled for T3 via `T3_GATED_PASSES` in `session.rs`
until each is independently verified GC-safe.  Unlock each by removing it
from `T3_GATED_PASSES` after adding a regression fixture that proves no Wasm
validation failure occurs across the `tests/fixtures/` suite.

| Pass | Reason unsafe for T3 | Unlock condition |
|------|----------------------|-----------------|
| `escape_analysis` | SROA unboxes GC-managed struct allocations → T3 emitter receives scalars instead of GC struct refs; Wasm validation fails | Teach SROA to skip GC-typed allocations (`StructInit` where type is GC-managed) |
| `type_narrowing` | Narrows `i64` → `i32`; T3 may use i64 for GC operand widths or struct field offsets; narrowing corrupts downstream T3 emitter expectations | Verify T3 emitter uses explicit type coercions rather than relying on local type for GC ops |
| `loop_unroll` | Unrolled loop bodies may contain GC allocations whose lifetime / drop order is altered in ways the T3 GC runtime does not expect | Audit T3 loop unroll interaction with GC allocation paths |
| `licm` | Hoisting GC allocations out of loops changes allocation site semantics; the T3 GC runtime may not track hoisted GC roots correctly | Add GC-root lifetime tracking to LICM before re-enabling for T3 |
| `bounds_check_elim` | GC array accesses in T3 Wasm are bounds-checked at the Wasm engine level; removing MIR-level checks changes observable trapping behavior | Verify T3 arrays use engine-level trapping and add invariant annotation before removing MIR checks |
| `inline_small_leaf` | Inlining changes GC object lifetime patterns — objects that were dropped at callee return are now kept alive until the caller's frame exits | Add GC-aware liveness check before inlining a callee that allocates GC objects |
| `aggregate_simplify` | Collapses single-element aggregates; if the aggregate is a GC struct, collapsing it removes the heap allocation the T3 emitter expects | Teach pass to skip GC-typed single-field structs |
| `gc_hint` | Annotates GC allocation sites for the runtime; T3 GC runtime does not yet consume these annotations, so they may produce invalid Wasm | Wire T3 GC runtime to consume `gc_hint` annotations, then re-enable |
| `branch_hint_infer` | O3 hint pass; T3 Wasm engine ignores branch hints — no benefit and untested for T3 | Re-evaluate when T3 target gains profile-guided execution support |

