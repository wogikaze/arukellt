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

