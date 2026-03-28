# MIR Optimization Passes

This directory establishes the convention for MIR optimization passes.
Each pass is a self-contained module.

## Current passes (in `opt/pipeline.rs`)

| Pass | Level | Description |
|------|-------|-------------|
| `const_fold` | O1 | Evaluate constant binary operations at compile time |
| `branch_fold` | O1 | Replace `if(true/false)` with unconditional goto |
| `cfg_simplify` | O1 | Simplify empty goto-only blocks |
| `copy_prop` | O1 | Replace `x = y` chains with direct source reference |
| `const_prop` | O1 | Substitute known constant values into operands |
| `dead_local_elim` | O1 | Remove unused local variables |
| `dead_block_elim` | O1 | Remove unreachable basic blocks |
| `unreachable_cleanup` | O1 | Remove code after unreachable terminators |
| `inline_small_leaf` | O2 | Inline small leaf functions (≤8 stmts, no calls) |
| `string_concat_opt` | O2 | Normalize string concatenation patterns |
| `aggregate_simplify` | O2 | Simplify single-element aggregates to plain use |
| `algebraic_simplify` | O2 | Identity/absorbing element elimination |
| `strength_reduction` | O2 | Replace mul/div by power-of-2 with shifts |

## Future passes (planned)

| Pass | Issue | Description |
|------|-------|-------------|
| `licm` | #080 | Loop-Invariant Code Motion |
| `escape_analysis` | #081 | Escape Analysis + Scalar Replacement |
| `gc_hint` | #082 | GC allocation hint annotations |
| `loop_unrolling` | #083 | Fixed-iteration loop unrolling |
| `cse` | #085 | Common Subexpression Elimination |
| `inter_fn_inline` | #087 | Inter-function inlining with dynamic thresholds |

## Adding a new pass

1. Create a new file in this directory: `passes/<name>.rs`
2. Add the pass variant to `OptimizationPass` enum in `opt/pipeline.rs`
3. Add `as_str()`, `DEFAULT_PASS_ORDER`, `OptimizationSummary` field, `absorb()`, `changed()`, `run_pass()` dispatch
4. Add tests in `pipeline.rs` or a dedicated test module
