# Benchmark Feature Matrix

Cross-reference of benchmarks against language features they exercise.

## Feature Matrix

| Benchmark | Category | Recursion | Loops | Structs | Vec | String | Closures | Match | Enum/Option | Functions | I/O (stdio) | Mutation |
|-----------|----------|:---------:|:-----:|:-------:|:---:|:------:|:--------:|:-----:|:-----------:|:---------:|:----------:|:--------:|
| `fib.ark` | compute | — | ✔ | — | — | — | — | — | — | ✔ | ✔ | ✔ |
| `binary_tree.ark` | compute | ✔ | — | — | — | — | — | — | — | ✔ | ✔ | — |
| `vec_ops.ark` | data-structure | — | ✔ | — | ✔ | — | — | — | — | ✔ | ✔ | ✔ |
| `vec-ops.ark` (legacy) | data-structure | — | ✔ | — | ✔ | — | — | ✔ | ✔ | ✔ | — | ✔ |
| `string_concat.ark` | string | — | ✔ | — | — | ✔ | — | — | — | ✔ | ✔ | ✔ |
| `string-ops.ark` (legacy) | string | — | — | — | — | ✔ | — | — | — | ✔ | — | — |
| `struct-create.ark` (legacy) | memory | — | — | ✔ | — | — | — | — | — | ✔ | — | — |
| `bench_cpu_enum_dispatch.ark` | compute | — | ✔ | — | — | — | — | ✔ | ✔ | ✔ | ✔ | ✔ |
| `parser.ark` (sample) | application | ✔ | ✔ | ✔ | ✔ | ✔ | — | ✔ | ✔ | ✔ | ✔ | ✔ |

### Legend

- **Category**: `compute` = CPU-bound arithmetic/logic; `data-structure` = collection allocation and traversal; `string` = string construction/manipulation; `memory` = allocation patterns; `application` = multi-feature real-world workload.
- ✔ = feature exercised; — = not exercised.

## Coverage Gaps

The following language features have **no dedicated benchmark**:

| Feature | Current coverage | Gap severity |
|---------|-----------------|:------------:|
| **Closures / higher-order functions** | None | High |
| **Enum / pattern matching** | `bench_cpu_enum_dispatch.ark` (dedicated dispatch benchmark) | Covered |
| **Error paths / Result handling** | None | Medium |
| **I/O-heavy workloads** | Only trivial `println` calls | Medium |
| **Struct-heavy allocation** | Only `struct-create.ark` (legacy, trivial) | Medium |
| **Nested structs / composite types** | Only `parser.ark` (not in bench suite) | Medium |
| **Host function calls (FFI)** | Only `stdio::println` | Low |
| **Imports / modules** | Only `use std::host::stdio` | Low |

## Suggested New Benchmarks

| Proposed benchmark | Category | Primary features | Gap filled |
|--------------------|----------|-----------------|------------|
| `bench_closure_map.ark` | compute | Closures, higher-order functions, Vec | Closures |
| `bench_enum_dispatch.ark` | compute | Enum variants, match, function dispatch | Enum / pattern matching |
| `bench_struct_graph.ark` | memory | Nested structs, recursive types, allocation | Struct-heavy |
| `bench_error_chain.ark` | compute | Result, error propagation, match | Error paths |
| `bench_file_io.ark` | io | File read/write, String, I/O host calls | I/O-heavy |
| `bench_parser_lite.ark` | application | String, structs, enums, loops, recursion | Multi-feature application |

### Priority

1. **`bench_closure_map.ark`** — closures are a core language feature with zero coverage.
2. **`bench_enum_dispatch.ark`** — enums and match are pervasive but untested for performance.
3. **`bench_struct_graph.ark`** — allocation patterns for composite types need measurement.
