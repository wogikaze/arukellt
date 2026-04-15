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
| `vec_push_pop.ark` | `allocation-heavy`, `container`, `gc-pressure` | Vec push (100k) then pop (100k); allocation + deallocation round-trip; stack-like access pattern |
| `vec-ops.ark` (removed) | `container`, `match` | Vec push (100), indexed get via `Option` match; basic container + pattern matching |

**Characteristics**: allocation-pressure sensitive, exercises GC or allocator, measures container API overhead.

### 3. String Benchmarks — String / GC Pressure

Workloads focused on string construction and manipulation.

| Benchmark | Tags | What it measures |
|-----------|------|-----------------|
| `string_concat.ark` | `string-heavy`, `allocation-heavy`, `gc-pressure` | Repeated concat (100 iterations); string buffer growth; allocation churn |
| `string_ops.ark` | `string`, `basic` | Single concat + len; baseline string API cost |

**Characteristics**: allocation-heavy, GC-sensitive, measures string representation efficiency.

### 4. Application Benchmarks — Multi-Feature / Real-World

Larger programs exercising multiple language features together.

| Benchmark | Tags | What it measures |
|-----------|------|-----------------|
| `parser.ark` (in `docs/sample/`) | `application`, `string-heavy`, `struct-heavy`, `recursion`, `match` | 1458-line Gloss markup parser; structs, enums, match, recursion, string ops, Vec — closest to real-world workload |
| `json_parse.ark` | `application`, `string-heavy`, `io-bound`, `host-call` | Reads a JSON file via `fs::read_to_string`; scans for character occurrences; exercises file I/O + string parsing |
| `bench_parse_tree_distance.ark` | `application`, `match`, `container`, `io-bound`, `allocation-heavy` | Reads a 1200-node distance matrix from file; parses and processes; exercises I/O, Vec, Result, enums, and match in a realistic workload |

**Characteristics**: representative of actual programs; exercises compiler optimizations across features; compile-time sensitive due to code size.

### 5. Legacy / Minimal Fixtures

Small fixtures used for correctness validation rather than performance measurement.

| Benchmark | Tags | What it measures |
|-----------|------|-----------------|
| `struct_create.ark` | `struct`, `basic` | Single struct creation + field access; minimal allocation |

**Characteristics**: too simple for meaningful perf measurement; useful for correctness parity checks.

### 6. Startup / Overhead Baseline

Benchmarks that isolate WASM module instantiation and process startup from computation.

| Benchmark | Tags | What it measures |
|-----------|------|------------------|
| `startup.ark` | `startup`, `overhead-baseline` | Minimal `main` with no logic; measures irreducible WASM instantiation + runtime startup overhead |

**Characteristics**: no computation, no allocation; establishes the minimum fixed overhead of any Arukellt program invocation.

## Feature Coverage Matrix

Per-benchmark view of which language and runtime features are exercised.

**Legend**: ● = primary exercise &nbsp;|&nbsp; ○ = secondary / incidental &nbsp;|&nbsp; – = not present

| Benchmark | loops | recursion | structs | enums/match | closures | higher-order | generics | strings | Vec | host-call | Result/Option | gc-pressure |
|-----------|:-----:|:---------:|:-------:|:-----------:|:--------:|:------------:|:--------:|:-------:|:---:|:---------:|:-------------:|:-----------:|
| `fib.ark` | ● | – | – | – | – | – | – | ○ | – | ○ | – | – |
| `binary_tree.ark` | – | ● | – | – | – | – | – | ○ | – | ○ | – | – |
| `vec_ops.ark` | ● | – | – | – | – | – | ○ | ○ | ● | ● | – | ○ |
| `vec_push_pop.ark` | ● | – | – | – | – | – | ○ | – | ● | ● | ○ | ● |
| `string_concat.ark` | ● | – | – | – | – | – | – | ● | – | ● | – | ● |
| `string_ops.ark` | – | – | – | – | – | – | – | ● | – | ○ | – | – |
| `struct_create.ark` | – | – | ● | – | – | – | – | ○ | – | ○ | – | – |
| `json_parse.ark` | ● | – | – | – | – | – | – | ● | – | ● | – | ○ |
| `startup.ark` | – | – | – | – | – | – | – | – | – | – | – | – |
| `bench_parse_tree_distance.ark` | ● | – | – | ● | – | – | ○ | ○ | ● | ● | ● | ○ |
| **Coverage count** | **7** | **1** | **1** | **1** | **0** | **0** | **2** | **7** | **4** | **8** | **1** | **3** |

> "Coverage count" = number of benchmarks where the feature is primary (●) or secondary (○).

## Workload Dimension Coverage

Per-benchmark view across the five operational workload dimensions.

**Legend**: ● = primary dimension &nbsp;|&nbsp; ○ = secondary / incidental &nbsp;|&nbsp; – = negligible

| Benchmark | compute | memory | I/O | GC | startup |
|-----------|:-------:|:------:|:---:|:--:|:-------:|
| `fib.ark` | ● | – | – | – | – |
| `binary_tree.ark` | ● | ○ | – | – | – |
| `vec_ops.ark` | ○ | ● | – | – | – |
| `vec_push_pop.ark` | ○ | ● | – | ● | – |
| `string_concat.ark` | – | ● | – | ● | – |
| `string_ops.ark` | – | ○ | – | – | – |
| `struct_create.ark` | – | ○ | – | – | – |
| `json_parse.ark` | ○ | ○ | ● | – | – |
| `startup.ark` | – | – | – | – | ● |
| `bench_parse_tree_distance.ark` | ● | ● | ○ | ○ | – |
| **Coverage count** | **5** | **7** | **2** | **2** | **1** |

**Dimension definitions**:

| Dimension | Meaning | Implied by tags |
|-----------|---------|------------------|
| `compute` | CPU-bound arithmetic, loops, deep recursion | `cpu-bound`, `recursion-heavy`, `loop` |
| `memory` | Heap allocation dominates the cost | `allocation-heavy`, `container`, `struct-heavy` |
| `I/O` | File or stream reads/writes via host calls | `io-bound`, `host-call` (when used for file access) |
| `GC` | Allocation churn that stresses the GC or allocator | `gc-pressure`, `string-heavy` |
| `startup` | WASM module instantiation + process startup cost | `startup`, `overhead-baseline` |

### Dimension gap summary

| Dimension | Coverage | Gap |
|-----------|:--------:|-----|
| compute | 5 benchmarks | Iterative and recursive well-covered; no SIMD or floating-point |
| memory | 7 benchmarks | Good coverage; struct-graph and pointer-chasing workloads missing |
| I/O | 2 benchmarks | **Gap**: only file-read path covered; no write or streaming benchmark |
| GC | 2 benchmarks | GC churn exercised by string and vec-push-pop only |
| startup | 1 benchmark | Sole baseline; no parameterised or multi-module startup variant |

### Feature gap summary

Features with zero or near-zero coverage:

| Feature | Coverage | Gap severity | Notes |
|---------|:--------:|:------------:|-------|
| **closures** | 0 | **critical** | No benchmark exercises closure literals at all |
| **higher-order fns** | 0 | **critical** | No benchmark passes or returns functions |
| **enums/match (custom)** | 1 | high | Only `bench_parse_tree_distance` uses `Result`; no custom enum dispatch |
| **recursion** | 1 | high | Only `binary_tree` is recursion-primary; no mutual recursion, no tail-call check |
| **structs under pressure** | 1 | medium | `struct_create` is smoke-level only; no large struct graphs or field-heavy iteration |
| **generics (user-defined)** | 0 | medium | All generic use is built-in `Vec<i32>`; no user-defined generic fns or types |
| **I/O-heavy loops** | 0 | medium | `fs::read_to_string` appears once but is not looped; no write path |
| **compile-stress** | 0 | low | No large polymorphic or macro-expanded sources |
| **error propagation** | 1 | low | `Result` is matched once; no chained propagation in a hot path |

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
| `io-bound` | File or stream I/O dominates cost |
| `startup` | Measures WASM instantiation / process startup |
| `overhead-baseline` | Establishes irreducible per-invocation overhead |
