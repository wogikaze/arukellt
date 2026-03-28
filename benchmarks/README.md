# Benchmarks

Benchmark suite for Arukellt language performance measurement.

## Quick Start

```bash
# Build release binary
cargo build --release -p arukellt

# Run full benchmark suite
bash benchmarks/run_benchmarks.sh

# Quick run (skip hyperfine, single time measurement)
bash benchmarks/run_benchmarks.sh --quick
```

## Benchmarks

| Benchmark | File | Description | Measures |
|-----------|------|-------------|----------|
| fib | `fib.ark` | Iterative Fibonacci(35) | CPU, loop overhead |
| binary_tree | `binary_tree.ark` | Recursive node counting (depth 20) | Function-call overhead, recursion |
| vec_ops | `vec_ops.ark` | Vec push/sum/contains (1k elements) | Memory allocation, iteration |
| string_concat | `string_concat.ark` | String concat in loop (100 iterations) | String allocation, GC pressure |

### Legacy Fixtures

| Fixture | File | Description |
|---------|------|-------------|
| vec-ops | `vec-ops.ark` | Basic Vec push/get/len |
| string-ops | `string-ops.ark` | Basic string concat |
| struct-create | `struct-create.ark` | Struct field creation |

## Tools

- **`run_benchmarks.sh`** — Compile, verify correctness, report binary sizes, and time execution.
  Uses [hyperfine](https://github.com/sharkdp/hyperfine) if available; falls back to `time`.
- **`parity-check.sh`** — Verify T1 vs T3 produce identical output for all `.ark` fixtures.
- **`size-compare.sh`** — Compare Wasm binary sizes between T1 and T3.

## Expected Output

Each benchmark has a `.expected` file containing the correct stdout.
`run_benchmarks.sh` verifies output against these files when `wasmtime` is available.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ARUKELLT` | `target/release/arukellt` | Path to compiler binary |
