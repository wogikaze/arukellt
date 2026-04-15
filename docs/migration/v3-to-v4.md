# Migration Guide: v3 → v4 (Optimization)

> Updated: 2026-04-15
> **Current-first note**: this guide explains the v3→v4 optimization and language refinement transition. For the current support matrix and known limitations, also check [`../current-state.md`](../current-state.md).

## Overview

v4 adds optimization passes and language refinements on top of the v3 stdlib base. Existing v3 code continues to compile and run; the compiler may produce smaller or faster output due to new optimization passes, but observable behavior is unchanged for correct programs.

Key additions in v4:
- **MIR opt-level separation** (issue #122): `--opt-level` flag, per-pass `passes/` directory with unified interface
- **T3 GC-safe pass audit** (issue #486): T3 (`wasm32-wasi-p2`) MIR optimization selectively re-enabled; `escape_analysis` and other potentially-unsafe passes remain gated
- Wasm binary size reduction (dead-code elimination, constant folding)
- Benchmark suite and stable performance baseline
- Additional language surface for method-syntax (`obj.method()`) on select builtin types (provisional)

## Changes

### Optimization passes (issue #122)

The v4 MIR pipeline adds optional optimization stages controlled by `--opt-level`:

```bash
arukellt compile --opt-level 1 myapp.ark   # default — 9 safe passes + DCE
arukellt compile --opt-level 2 myapp.ark   # all 20 passes + inter-function inline
arukellt compile --opt-level 0 myapp.ark   # no optimization (debug)
```

Individual passes can be disabled with `--no-pass <name>` (e.g. `--no-pass cse`).

Optimization does **not** change language semantics for well-formed programs.

**T3 note (issue #486)**: On `wasm32-wasi-p2`, all O1 passes are active at O1+. However,
`escape_analysis`, `type_narrowing`, `loop_unroll`, `inline_small_leaf`, and several other
O2 passes remain **disabled for T3** until each is verified GC-safe. Use `--target wasm32-wasi-p1`
if those optimizations are required. Dead function elimination is also disabled for T3.

### Wasm size reduction

Dead code elimination and constant folding are applied before Wasm emit. Generated `.wasm` sizes will typically be smaller than v3 output for the same source.

### Benchmark suite

`benchmarks/` contains the v4 benchmark suite. A stable baseline is captured in `docs/process/benchmark-results.md`. Performance regressions against this baseline are flagged by CI.

### Method-syntax preview (provisional)

Select builtin types support `.method()` call syntax as a provisional feature:

```arukellt
let s = "hello";
let n = s.len();     // provisional — equivalent to string_len(s)
```

This surface is `provisional` and may change in v5. Avoid relying on method syntax in library code intended to be stable across versions.

## Migration Steps

1. **Update toolchain** — `mise install` to get the v4 compiler.

2. **Verify output correctness** — if your code relies on a specific Wasm binary layout or size, re-test after upgrading, as optimization may change the binary.

3. **Adjust opt-level if needed** — pass `--opt-level 0` to disable optimization for debugging or reproducible builds.

4. **Check benchmark baselines** — if you track performance, re-establish baselines against v4 output.

5. **Avoid provisional method syntax in stable libraries** — prefer function-call form for APIs intended to remain stable into v5.

## Unchanged Behavior

- All v3 stdlib imports (`use std::*`) and the module system continue to work.
- All v2 CLI flags (`--emit component`, `--emit wit`, etc.) continue to work.
- T1 (`wasm32-wasi-p1`) compatibility path remains available.
- Language syntax for functions, structs, enums, and generics is unchanged.
- Stability labels and deprecated API warnings from v3 remain in effect.

## Known Limitations

- `escape_analysis` and several O2 passes are disabled for T3 (`wasm32-wasi-p2`) — they are not yet GC-safe. See `crates/ark-mir/src/passes/README.md` for per-pass unlock conditions.
- Dead function elimination is disabled for T3 (WASI export reachability concern).
- `--opt-level 2` may increase compile time significantly for large inputs.
- Method syntax is provisional and only available on select builtin types.
- Async / streaming Component Model features remain out of scope (planned for v5+).
- The `binary_tree` benchmark (deep recursion + dense heap allocation) shows 1.83x overhead on T3 vs T1. This is expected GC overhead for this workload class; see [`../process/benchmark-results.md`](../process/benchmark-results.md) for details and workaround.

## See Also

- [`docs/process/benchmark-results.md`](../process/benchmark-results.md) — stable performance baselines
- [`docs/current-state.md`](../current-state.md) — current support matrix
- [`v2-to-v3.md`](v2-to-v3.md) — previous migration guide
- [`v4-to-v5.md`](v4-to-v5.md) — next migration guide
