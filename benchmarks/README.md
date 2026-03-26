# Benchmarks

Benchmark fixtures for comparing T1 and T3 targets.

## Running Benchmarks

```bash
# Build release binary first
cargo build --release

# Run parity check (T1 vs T3 output comparison)
bash benchmarks/parity-check.sh

# Run size comparison
bash benchmarks/size-compare.sh
```

## Fixtures

| Fixture | Description | Measures |
|---------|-------------|----------|
| `fib.ark` | Fibonacci computation | CPU, correctness |
| `vec-ops.ark` | Vec push/get/len | Memory, correctness |
| `string-ops.ark` | String operations | Memory, correctness |
| `struct-create.ark` | Struct creation | Memory, correctness |

## Parity Probe

The parity check verifies that T1 and T3 produce identical stdout for each fixture.
This ensures language semantics are target-independent.
