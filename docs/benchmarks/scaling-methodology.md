# Scaling Curve Methodology

How input-size sweeps reveal algorithmic complexity and detect performance
cliffs in the Arukellt benchmark suite.

## Why Scaling Curves?

A single-point benchmark answers "how fast is this?" but not "how does
performance change as input grows?" Scaling curves answer the second
question by running the same workload at multiple input sizes and plotting
size vs. cost.

This matters because:

- **Algorithmic regressions hide at small N.** A quadratic routine may
  look fine at N=100 but collapse at N=10 000.
- **GC and allocator cliffs** appear when a threshold is crossed (e.g.,
  nursery overflow, realloc doubling).
- **Constant-factor improvements are visible** as vertical shifts in the
  curve without changing its shape.

## Running a Scaling Sweep

Use `mise bench` (full mode) to capture timing data across multiple iterations,
then compare results across runs stored in `benchmarks/results/` to observe
scaling behaviour.

```bash
mise bench   # full mode (10 iterations)
```

Output conforms to `arukellt-bench-v1` schema. Per-size analysis can be done
by inspecting the JSON results directly or by running
`python3 scripts/util/benchmark_runner.py --mode compare`.

## Benchmark Scaling Parameters

Each benchmark has a single "knob" that controls input size:

| Benchmark       | Parameter             | Source constant               | Default |
|-----------------|-----------------------|-------------------------------|---------|
| `fib`           | Sequence length       | `fib(35)` in `fib.ark`       | 35      |
| `binary_tree`   | Tree recursion depth  | `let depth: i32 = 20`        | 20      |
| `vec_ops`       | Vector element count  | `while i < 1000`             | 1000    |
| `string_concat` | Concatenation iters   | `while i < 100`              | 100     |

The scaling script generates temporary variants by rewriting these
constants via `sed`, compiles each variant, and times the execution.

## Expected Scaling Behavior

| Benchmark       | Expected complexity | Rationale                                    |
|-----------------|---------------------|----------------------------------------------|
| `fib`           | **O(n)** iterative  | Linear loop from 0 to N; each step is O(1).  |
| `binary_tree`   | **O(2ⁿ)** exponential | Full binary recursion doubles work per depth level. |
| `vec_ops`       | **O(n)** linear     | Single pass push + single pass sum; both linear in element count. |
| `string_concat` | **O(n)** to **O(n²)** | Depends on string representation. Immutable concat copies prior content → quadratic; rope/buffer → linear. |

> **Note:** `fib.ark` uses an iterative algorithm, so despite computing
> Fibonacci numbers its runtime scales linearly. `binary_tree.ark` uses
> naive recursive doubling, making it the exponential workload in the suite.

## Interpreting Complexity from Data

### Growth Ratio Method

The simplest approach: compare the ratio of runtimes between consecutive
size points.

```
ratio = T(n₂) / T(n₁)
```

| Observed ratio pattern                  | Likely complexity |
|-----------------------------------------|-------------------|
| ratio ≈ constant (≈1–2×)               | **O(n)** linear   |
| ratio ≈ (n₂/n₁)²                       | **O(n²)** quadratic |
| ratio doubles when input increments +1  | **O(2ⁿ)** exponential |
| ratio ≈ 1 regardless of size            | **O(1)** constant  |

**Example — binary_tree (exponential):**

| Depth | Median (ms) | Ratio vs. prev |
|-------|-------------|----------------|
| 5     | 0.4         | —              |
| 10    | 2.1         | 5.3×           |
| 15    | 58          | 27.6×          |
| 20    | 1 820       | 31.4×          |

Each +5 depth roughly multiplies time by 2⁵ = 32, confirming O(2ⁿ).

**Example — vec_ops (linear):**

| Size   | Median (ms) | Ratio vs. prev |
|--------|-------------|----------------|
| 100    | 1.2         | —              |
| 1 000  | 2.8         | 2.3×           |
| 10 000 | 18.5        | 6.6×           |

A 10× size increase yields roughly 6–7× time increase (close to 10×
after subtracting startup overhead), consistent with O(n).

### Log-Log Regression

For more precision, plot log(size) vs. log(time). The slope of the
best-fit line gives the exponent:

- slope ≈ 1 → O(n)
- slope ≈ 2 → O(n²)
- slope grows with n → exponential (not polynomial)

### Cliff Detection

The scaling script flags a **cliff warning** when `ratio > 10×` between
adjacent size points. Common causes:

- **Algorithmic blowup**: e.g., accidentally switching from iterative to
  recursive Fibonacci
- **GC cliff**: nursery or heap promotion threshold crossed
- **Allocator realloc cascade**: vector backing storage triggers large
  copies
- **Cache effects**: working set exceeds L1/L2/L3 cache

When a cliff is detected, investigate by:

1. Narrowing the sweep range around the cliff point
2. Checking memory metrics (`mise bench` — memory fields in JSON results)
3. Reviewing recent code changes for algorithmic regressions

## Integration with CI

The scaling sweep is not part of the default CI gate (it is too slow for
every push). Recommended usage:

- **Pre-release**: run full sweeps for all benchmarks before tagging a
  version
- **Regression investigation**: run a targeted sweep when a single-point
  benchmark shows unexpected regression
- **Optimizer validation**: compare scaling curves before/after an
  optimization pass to verify the expected complexity improvement

## References

- [`benchmarks/README.md`](../../benchmarks/README.md) — benchmark suite overview
- [`docs/benchmarks/governance.md`](governance.md) — naming, schema, comparison methodology
- [`docs/benchmarks/variance-control.md`](variance-control.md) — controlling measurement noise
