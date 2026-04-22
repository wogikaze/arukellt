# Optimization Passes

> Current-first reference for the v4 optimization pipeline.
> For pipeline context see [pipeline.md](pipeline.md); for overall state see [../current-state.md](../current-state.md).

## Overview

Optimizations run at two levels:

1. **MIR level** — target-independent passes applied before backend emission
2. **Backend level** — target-specific peephole / layout optimizations applied during Wasm emission

Both layers are gated by `--opt-level` (0, 1, 2). The default is `--opt-level 1`.

## `--opt-level` Behavior

| Level | MIR Passes | Dead Function Elimination | Backend Peephole | Backend Layout |
|-------|-----------|--------------------------|------------------|----------------|
| `0`   | None      | Disabled                 | Disabled         | Disabled       |
| `1`   | Safe subset (9 passes) | Enabled        | Enabled          | Disabled       |
| `2`   | All 20 passes (up to 3 rounds) | Enabled | Enabled          | Enabled (struct field reorder) |

Environment variable `ARUKELLT_NO_DEAD_FN=1` disables dead function elimination at any level.

Individual passes can be disabled with `--no-pass <name>` (e.g. `--no-pass cse`).

## MIR Optimization Passes

All MIR passes live in selfhost `src/compiler/mir.ark` (the prior Rust `crates/ark-mir/src/opt/` was retired in #561). The pipeline runs up to 3 fixed-point
rounds; iteration stops early when a round produces no changes.

### Default pass order

The following table lists passes in their execution order within each round.
O1 runs only the subset marked ✓ in the O1 column; O2 runs all.

| # | Pass | O1 | Description |
|---|------|----|-------------|
| 1 | `const_fold` | ✓ | Evaluate constant binary operations at compile time |
| 2 | `branch_fold` | ✓ | Replace `if (true)` / `if (false)` branches with unconditional gotos |
| 3 | `cfg_simplify` | ✓ | Remove empty basic blocks that consist only of a `goto` |
| 4 | `loop_unroll` | | Unroll small loops (≤ 8 stmts body, ≤ 4 iterations) with statically known bounds |
| 5 | `copy_prop` | ✓ | Propagate `x = y` assignments, replacing uses of `x` with `y` |
| 6 | `const_prop` | ✓ | Propagate constant definitions to all uses within a basic block |
| 7 | `type_narrowing` | | Narrow `i64` locals to `i32` when values provably fit in 32-bit range |
| 8 | `escape_analysis` | | Scalar replacement of aggregates — replace non-escaping struct allocations with individual locals |
| 9 | `bounds_check_elim` | | Remove redundant array bounds checks when the index is provably in range |
| 10 | `dead_local_elim` | ✓ | Remove unused local variables |
| 11 | `dead_block_elim` | ✓ | Remove unreachable basic blocks (no predecessor path from entry) |
| 12 | `unreachable_cleanup` | ✓ | Trim statements after unconditional `return` within a block |
| 13 | `inline_small_leaf` | | Inline leaf functions ≤ 8 stmts with no calls (intra-function); inter-function inline runs at module level for leaves ≤ 20 stmts called ≤ 3 times |
| 14 | `string_concat_opt` | | Normalize `concat(a, b)` call patterns for downstream lowering |
| 15 | `aggregate_simplify` | | Simplify single-element aggregate constructors to plain assignments |
| 16 | `algebraic_simplify` | | Identity/absorbing-element elimination (`x + 0 → x`, `x * 1 → x`, `x * 0 → 0`, `!!x → x`, etc.) |
| 17 | `strength_reduction` | | Replace expensive ops with cheaper equivalents (`x * 2^n → x << n`, `x / 2^n → x >> n`) |
| 18 | `cse` | ✓ | Common subexpression elimination within basic blocks |
| 19 | `gc_hint` | | Annotate short-lived struct allocations inside loops with `GcHint::ShortLived` for downstream GC |
| 20 | `branch_hint_infer` | | Infer branch likelihood hints (mark error/panic paths as unlikely) |

### Module-level passes (outside the per-function round loop)

| Pass | Gate | Description |
|------|------|-------------|
| Inter-function inline | O2, before rounds | Inline small leaf functions (≤ 20 stmts, called ≤ 3 times) into callers across function boundaries |
| Dead function elimination | O1+, after rounds | BFS from entry + exported functions; remove unreachable functions (primarily stdlib dead code) |

### Validation

Every individual pass is bracketed by MIR validation (`validate_module`) — the module is
validated before and after each pass. This ensures pass bugs are caught immediately rather
than surfacing as backend errors.

## Backend Optimizations (T3 — `wasm32-wasi-p2`)

Backend optimizations are applied during Wasm emission in the selfhost emitter
(`src/compiler/emitter.ark`).

### Peephole optimization (`peephole.rs`)

The `PeepholeWriter` wraps `wasm_encoder::Function` and intercepts instruction emission:

| Pattern | Replacement | Gate |
|---------|------------|------|
| `local.set X ; local.get X` | `local.tee X` | `opt_level >= 1` |

This avoids redundant store-then-load by keeping the value on the Wasm stack while also
writing it to the local variable.

### Struct field layout optimization (`layout_opt.rs`)

At `opt_level >= 2`, the emitter reorders struct fields by descending access frequency
so that hot fields get lower Wasm GC field indices, improving JIT locality.

- Only structs with ≥ 3 fields are considered
- Frequency is measured by scanning all reachable MIR functions
- Ties preserve declaration order (stable sort)

### Dead code elimination at backend level (`reachability.rs`)

The T3 emitter performs its own function-level reachability analysis:

- Entry point + exported (pub) functions are roots
- Transitive call-graph walk determines reachable functions
- Only reachable functions are emitted to the final Wasm module
- WASI imports (e.g. `path_open`, `fd_read`) are conditionally included only when the
  reachable code uses filesystem builtins

### Read-modify-write code generation

The T3 backend already generates optimal RMW sequences for struct field updates (6 instructions
per field update — the minimum for a stack machine without `dup`). See [t3-rmw-optimization.md](t3-rmw-optimization.md) for the analysis.

## Binary Size Optimizations

Binary size reductions come from the combined effect of:

1. **Dead function elimination** — stdlib functions not reachable from user code are removed
2. **Backend reachability** — only reachable functions are emitted
3. **GC-native representation** — T3 avoids linear-memory allocator overhead (no bump allocator, heap pointer, or GC runtime)
4. **Conditional WASI imports** — filesystem-related imports are omitted when unused
5. **Peephole `local.tee`** — reduces instruction count

T3 GC-native binaries are 86–92% smaller than T1 equivalents for typical programs.

## Constant Folding / Propagation Status

Both passes are **implemented and active**:

- `const_fold` evaluates compile-time-known binary operations (arithmetic, comparison, logical) and replaces them with constant results
- `const_prop` propagates constant definitions forward within basic blocks, enabling further folding

Both run at O1 and O2. The combination with `branch_fold` enables dead branch elimination
when conditions become statically known after propagation.

## Dead Code Elimination Status

Dead code elimination operates at three levels:

| Level | Mechanism | Gate |
|-------|-----------|------|
| Statement | `unreachable_cleanup` removes stmts after `return` | O1+ |
| Block | `dead_block_elim` removes blocks with no predecessor path | O1+ |
| Local | `dead_local_elim` removes unused local variables | O1+ |
| Function (MIR) | `eliminate_dead_functions` removes unreachable functions | O1+ |
| Function (backend) | T3 reachability analysis emits only reachable functions | Always |

## Dump / Debugging Support

Set `ARUKELLT_DUMP_PHASES=optimized-mir` (or `all`) to see MIR state before optimization,
after each pass that produces changes, and after the final round.

```bash
ARUKELLT_DUMP_PHASES=optimized-mir arukellt compile hello.ark
```

## Related

- [pipeline.md](pipeline.md) — overall compilation pipeline
- [t3-rmw-optimization.md](t3-rmw-optimization.md) — RMW pattern analysis
- [ir-spec.md](ir-spec.md) — MIR specification
- [../current-state.md](../current-state.md) — project status
