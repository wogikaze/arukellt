# Migration Guide: v3 → v4 (Optimization)

> Updated: 2026-03-29

## Overview

v4 introduces a MIR-level optimization pipeline and backend-level Wasm improvements. The compiler gains `--opt-level` and `--time` flags, seven independent MIR optimization passes, and measurable improvements to compile time, runtime performance, memory usage, and binary size. Existing programs require no source changes; the optimizations are semantics-preserving.

## New CLI Flags

### `--opt-level 0|1|2`

Controls which optimization passes are applied:

| Level | Passes | Use case |
|-------|--------|----------|
| `0` | None | Debug builds, fastest compile |
| `1` | `const_folding`, `dce` | Default — safe optimizations only |
| `2` | All 7 passes | Release builds — maximum optimization |

```bash
arukellt compile --opt-level 2 myapp.ark
arukellt compile --opt-level 0 myapp.ark   # debug, no optimizations
```

Default is `--opt-level 1`. Use `--debug` as a shorthand for `--opt-level 0`.

### `--time`

Reports per-phase compilation time on stderr:

```bash
arukellt compile --time myapp.ark
# lex: 2ms, parse: 5ms, resolve: 8ms, typecheck: 15ms, mir: 3ms, optimize: 4ms, emit: 6ms
```

### `ARUKELLT_DUMP_PHASES=optimized-mir`

Dumps MIR before and after optimization passes to stderr:

```bash
ARUKELLT_DUMP_PHASES=optimized-mir arukellt compile myapp.ark
```

## MIR Optimization Passes

Seven passes are introduced in `crates/ark-mir/src/passes/`, each independently toggleable:

| Pass | `--opt-level` | Description |
|------|---------------|-------------|
| `const_folding` | ≥ 1 | Constant folding: `BinOp(Const(a), Add, Const(b))` → `Const(a+b)` |
| `dce` | ≥ 1 | Dead code elimination: unreachable blocks, unused locals |
| `copy_propagation` | ≥ 2 | Copy propagation: `let x = y; use(x)` → `use(y)` |
| `inline` | ≥ 2 | Inlining: single-call-site functions ≤ 10 instructions |
| `licm` | ≥ 2 | Loop-invariant code motion |
| `escape_analysis` | ≥ 2 | Escape analysis with scalar replacement for non-escaping structs |
| `gc_hint` | ≥ 2 | GC hints for short-lived objects (wasmtime-dependent, may be no-op) |

Individual passes can be disabled with `--no-pass=<name>`:

```bash
arukellt compile --opt-level 2 --no-pass=inline myapp.ark
```

## Backend Optimizations (T3)

The T3 Wasm emitter (`t3_wasm_gc.rs`) gains peephole optimizations:

- Redundant `local.get`/`local.set` pair elimination
- String literal deduplication in data segments
- Dead `if` branch removal for constant conditions
- `struct.get` + immediate `struct.set` pattern fusion

These are always applied (independent of `--opt-level`).

## Performance Targets

| Metric | Target |
|--------|--------|
| Compile time (`hello.ark`) | ≤ 50 ms |
| Compile time (500-line `parser.ark`) | ≤ 500 ms |
| Runtime `fib(35)` | ≤ 1.5× C (gcc -O2) |
| Runtime `vec-ops` (100k elements) | ≤ 2.0× C (gcc -O2) |
| Compiler RSS (1000-line input) | ≤ 100 MB |
| Binary size `hello.wasm` (GC-native) | ≤ 1 KB |
| Binary size `parser.wasm` (500-line) | ≤ 50 KB |

## Benchmarks

New benchmark suite in `benchmarks/`:

- `binary_tree.ark` (depth 15)
- `vec_push_pop.ark` (100k elements)
- `string_concat.ark` (10k iterations)

Run benchmarks:

```bash
scripts/run-benchmarks.sh --compare-lang c,rust,go
```

Baselines are stored in `tests/baselines/perf/` and checked by `scripts/verify-harness.sh`.

## Unchanged Behavior

- Source language syntax is unchanged.
- All v3 stdlib modules and APIs remain available.
- T1 and T3 compilation paths continue to work.
- Component Model features from v2 are unaffected.
- Programs produce identical output regardless of `--opt-level`.

## Migration Checklist

- [ ] No source changes required — v4 is fully backwards compatible
- [ ] (Optional) Add `--opt-level 2` to release build scripts
- [ ] (Optional) Add `--time` to CI to track compile-time regressions
- [ ] (Optional) Update benchmark baselines with `scripts/update-baselines.sh`
- [ ] (Optional) Use `ARUKELLT_DUMP_PHASES=optimized-mir` to inspect optimization effects

## Related Documents

- `docs/process/roadmap-v4.md` — historical v4 roadmap
- `docs/compiler/pipeline.md` — compiler pipeline and MIR pass documentation
- `benchmarks/README.md` — benchmark methodology and results
- `docs/current-state.md` — current project state
