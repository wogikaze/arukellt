# Benchmark Variance Control & Reproducibility

This document defines the variance-control strategy and reproducibility profile
for the Arukellt benchmark suite. It supplements the
[governance doc](governance.md) with guidance on obtaining stable, trustworthy
measurements.

## 1. Sources of Variance

| Source | Impact | Notes |
|--------|--------|-------|
| **OS scheduling** | High | Context switches inject unpredictable latency; the kernel may migrate a process between cores mid-run. |
| **Thermal throttling** | Medium–High | Sustained load causes CPUs to reduce clock speed (turbo boost decay). Early iterations run faster than later ones. |
| **Background processes** | Medium | Browser tabs, IDE indexers, package managers, and system daemons consume CPU and memory bandwidth. |
| **GC / allocator non-determinism** | Low–Medium | The Wasm runtime's internal allocator may coalesce or split differently across runs; large-heap benchmarks are most affected. |
| **Filesystem caching** | Low | First compilation may be slower if caches are cold; subsequent runs benefit from warm page cache. |
| **NUMA topology** | Low | On multi-socket machines, memory access latency depends on which node the process is scheduled to. |

## 2. Mitigation Strategies

### Fixed iteration count

Every benchmark mode defines an explicit iteration count (see
[governance §3](governance.md#3-execution-modes)). The `full` and `ci` modes
use **5 compile iterations** and **5 runtime iterations** to gather enough
samples for a stable median.

### Warmup runs

The first iterations pay cold-cache and JIT-warmup costs. The reproducibility
profile (§3) discards the first **2** iterations as warmup so that only
steady-state measurements contribute to the reported statistic.

### Median over min

We report the **median** rather than the minimum or mean.

- The **minimum** is biased toward best-case scheduling luck and is not
  reproducible.
- The **mean** is sensitive to outliers caused by background noise.
- The **median** is robust against both high and low outliers while remaining
  representative of typical performance.

### Process isolation

For trustworthy numbers:

- Close CPU-intensive applications (browsers, IDEs, builds) before running
  benchmarks.
- Disable automatic updates, indexers, and cron jobs where possible.
- On Linux, consider pinning to a single core with `taskset` or setting the CPU
  governor to `performance`:

  ```bash
  # Pin to core 0
  taskset -c 0 mise bench

  # Set CPU governor (requires root)
  echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
  ```

## 3. Reproducibility Profile

The **default reproducibility profile** applies to `full`, `compare`, `ci`, and
`update-baseline` modes:

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Total iterations | ≥ 5 | Enough samples to compute a meaningful median |
| Warmup (discarded) | 2 | Eliminates cold-cache and JIT-warmup effects |
| Reported statistic | Median | Robust to outliers (see §2) |
| Environment metadata | Recorded | CPU, kernel, rustc, wasmtime, opt-level captured per run |

### Workflow

1. Run **N** iterations (N ≥ 5).
2. **Discard** the first 2 as warmup.
3. From the remaining samples, compute the **median**.
4. Record environment metadata alongside the result.
5. If the **coefficient of variation (CV)** of the kept samples exceeds the
   acceptable threshold (§4), flag the measurement as **unstable**.

## 4. Acceptable Variance Thresholds

| Metric | Maximum CV | Rationale |
|--------|:----------:|-----------|
| Compile time | < 5 % | Compilation is CPU-bound and deterministic in work; variance above 5 % indicates environmental noise. |
| Runtime | < 10 % | Runtime exercises the Wasm engine and OS scheduler; slightly more variance is expected. |
| Binary size | < 1 % | Binary output is deterministic for a given compiler+flags; any variance signals a build reproducibility bug. |

When a benchmark exceeds its CV threshold:

- The measurement is flagged **unstable** in reports.
- CI treats unstable benchmarks as informational (no gate failure) unless the
  regression also exceeds the [regression threshold](governance.md#threshold-policy-ci-gate).
- To diagnose noise, run the benchmark with more iterations via `mise bench` and compare variance across runs (see §5).

## 5. Detecting Noisy Measurements

To detect instability, run the benchmark multiple times and observe variance:

```bash
# Run with 10 iterations
mise bench
```

Compare runs by looking at the CV (coefficient of variation) in the JSON results
stored in `benchmarks/results/`.

### Triage checklist for noisy benchmarks

1. **Re-run** with the system idle — close browsers, IDEs, and background
   builds.
2. **Pin the CPU governor** to `performance` (see §2).
3. **Increase iterations** (e.g., `BENCH_ITERATIONS=20`) to see whether the
   median converges.
4. **Check for thermal throttling** — monitor CPU frequency during the run
   (`watch -n1 cat /proc/cpuinfo | grep MHz`).
5. If the benchmark is inherently noisy (e.g., GC-heavy), consider adding a
   `gc-pressure` tag and raising the per-benchmark threshold in the runner
   configuration.

## 6. CI Considerations

### Shared runners

CI environments (GitHub Actions, etc.) run on shared hardware. Expect higher
variance than local measurements:

- **Increase iteration count** in CI mode if budget allows.
- **Never hard-fail on variance alone** — use variance as a signal, not a gate.
  The regression threshold (§4 of [governance](governance.md)) is the
  authoritative gate.
- **Record environment metadata** so that noisy CI runs can be attributed to
  specific machine types or load conditions.

### Caching

- Ensure the Rust toolchain and `wasmtime` binary are cached between CI runs to
  avoid cold-build variance in the benchmark harness itself.
- The `.ark → .wasm` compilation being benchmarked should **not** be cached —
  that is the workload under test.

### Baseline stability

- Update baselines only from a **known-good environment** (local machine or
  dedicated CI runner), not from shared runners with high variance.
- When comparing across commits, both baseline and current measurements should
  use the same runner class and iteration profile.

### Flaky-benchmark policy

If a benchmark consistently exceeds its CV threshold in CI:

1. Tag it `unstable` in the benchmark metadata.
2. Exclude it from the hard regression gate.
3. File an issue to investigate and stabilize or replace the fixture.
