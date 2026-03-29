# Workload Taxonomy

Classification of benchmark workloads by category, intent, and coverage status.

## Taxonomy

### 1. Micro Benchmarks — CPU / Scalar

Small kernels that stress arithmetic, branching, and loop overhead.

| Benchmark | Tags | What it measures |
|-----------|------|-----------------|
| `fib.ark` | `cpu-bound`, `loop`, `scalar` | Iterative loop throughput; integer arithmetic; branch prediction friendliness |
| `binary_tree.ark` | `recursion-heavy`, `call-heavy` | Deep recursion; call-frame allocation/deallocation; stack depth limits |

**Characteristics**: deterministic, no allocation beyond stack frames, easy to compare across compilers.

### 2. Data Structure Benchmarks — Collection / Allocation

Workloads dominated by heap allocation and container operations.

| Benchmark | Tags | What it measures |
|-----------|------|-----------------|
| `vec_ops.ark` | `allocation-heavy`, `container`, `iteration` | Vec push (1k), sequential read, `contains` search; allocation throughput and linear scan cost |
| `vec-ops.ark` (legacy) | `container`, `match` | Vec push (100), indexed get via `Option` match; basic container + pattern matching |

**Characteristics**: allocation-pressure sensitive, exercises GC or allocator, measures container API overhead.

### 3. String Benchmarks — String / GC Pressure

Workloads focused on string construction and manipulation.

| Benchmark | Tags | What it measures |
|-----------|------|-----------------|
| `string_concat.ark` | `string-heavy`, `allocation-heavy`, `gc-pressure` | Repeated concat (100 iterations); string buffer growth; allocation churn |
| `string-ops.ark` (legacy) | `string`, `basic` | Single concat + len; baseline string API cost |

**Characteristics**: allocation-heavy, GC-sensitive, measures string representation efficiency.

### 4. Application Benchmarks — Multi-Feature / Real-World

Larger programs exercising multiple language features together.

| Benchmark | Tags | What it measures |
|-----------|------|-----------------|
| `parser.ark` (in `docs/sample/`) | `application`, `string-heavy`, `struct-heavy`, `recursion`, `match` | 1458-line Gloss markup parser; structs, enums, match, recursion, string ops, Vec — closest to real-world workload |

**Characteristics**: representative of actual programs; exercises compiler optimizations across features; compile-time sensitive due to code size.

### 5. Legacy / Minimal Fixtures

Small fixtures used for correctness validation rather than performance measurement.

| Benchmark | Tags | What it measures |
|-----------|------|-----------------|
| `struct-create.ark` (legacy) | `struct`, `basic` | Single struct creation + field access; minimal allocation |

**Characteristics**: too simple for meaningful perf measurement; useful for correctness parity checks.

## Missing Categories

The following workload categories have **no benchmark coverage**:

| Category | Description | Why it matters |
|----------|-------------|---------------|
| **I/O-heavy** | File or stream read/write in a loop | Measures host-call overhead, buffering strategy, and I/O bridging cost |
| **Struct-heavy / Graph** | Large struct graphs, linked structures, tree traversal with data | Exercises heap layout, pointer chasing, and composite-type allocation |
| **Closure-heavy / Higher-order** | map/filter/fold patterns, callback chains | Measures closure capture cost, indirect-call dispatch, and inlining effectiveness |
| **Enum dispatch** | Large match on many variants, visitor patterns | Tests pattern-matching compilation strategy and branch-table efficiency |
| **Error-path** | Result propagation, error chains, recovery | Measures cost of error-handling paths vs happy paths |
| **Compile-stress** | Very large source files or deeply nested types | Measures compiler throughput independent of runtime performance |

## Tag Reference

Canonical tags for benchmark classification:

| Tag | Meaning |
|-----|---------|
| `cpu-bound` | Dominated by arithmetic / branching |
| `recursion-heavy` | Deep recursive call trees |
| `call-heavy` | High function-call count |
| `allocation-heavy` | Significant heap allocation |
| `container` | Exercises Vec or similar containers |
| `iteration` | Loop-based traversal |
| `string-heavy` | String construction / manipulation |
| `gc-pressure` | Creates allocation churn for GC |
| `struct-heavy` | Struct creation and field access |
| `match` | Pattern matching / enum dispatch |
| `application` | Multi-feature, real-world-like |
| `basic` | Minimal / smoke-test level |
