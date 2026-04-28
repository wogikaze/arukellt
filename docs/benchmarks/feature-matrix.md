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
| `bench_memory_struct_graph.ark` | memory | ✔ | ✔ | ✔ | — | — | — | — | — | ✔ | ✔ | — |
| `bench_compute_error_chain.ark` | compute | — | ✔ | — | — | — | — | ✔ | ✔ | ✔ | ✔ | ✔ |
| `bench_cpu_closure_map.ark` | compute | — | ✔ | — | ✔ | — | ✔ | — | — | ✔ | ✔ | ✔ |
| `bench_application_http_parser.ark` | application | — | ✔ | ✔ | — | — | — | ✔ | ✔ | ✔ | ✔ | ✔ |
| `bench_application_log_processor.ark` | application | — | ✔ | ✔ | ✔ | — | — | ✔ | ✔ | ✔ | ✔ | ✔ |
| `bench_application_config_loader.ark` | application | — | ✔ | ✔ | — | — | — | ✔ | ✔ | ✔ | ✔ | ✔ |
| `bench_application_data_pipeline.ark` | application | — | ✔ | ✔ | ✔ | — | ✔ | — | — | ✔ | ✔ | ✔ |
| `bench_application_template_engine.ark` | application | ✔ | — | — | — | ✔ | — | ✔ | ✔ | ✔ | ✔ | ✔ |
| `parser.ark` (sample) | application | ✔ | ✔ | ✔ | ✔ | ✔ | — | ✔ | ✔ | ✔ | ✔ | ✔ |

### Legend

- **Category**: `compute` = CPU-bound arithmetic/logic; `data-structure` = collection allocation and traversal; `string` = string construction/manipulation; `memory` = allocation patterns; `application` = multi-feature real-world workload.
- ✔ = feature exercised; — = not exercised.

## Coverage Gaps

The following language features have **no dedicated benchmark**:

| Feature | Current coverage | Gap severity |
|---------|-----------------|:------------:|
| **Closures / higher-order functions** | `bench_cpu_closure_map.ark` (dedicated closure benchmark), `bench_application_data_pipeline.ark` (pipeline filter/map closures) | Covered |
| **Enum / pattern matching** | `bench_cpu_enum_dispatch.ark` (dedicated dispatch benchmark) | Covered |
| **Error paths / Result handling** | `bench_compute_error_chain.ark` (Result / error propagation chain) | Covered |
| **HashMap / container operations** | `bench_application_template_engine.ark` (HashMap<String,String> CRUD, template expansion) | Covered |
| **I/O-heavy workloads** | Only trivial `println` calls | Medium |
| **Struct-heavy allocation** | `bench_memory_struct_graph.ark` (nested structs, recursive graph) | Covered |
| **Nested structs / composite types** | `bench_memory_struct_graph.ark` (Vec2→Rect→BBox, depth-10 recursion) | Covered |
| **Host function calls (FFI)** | Only `stdio::println` | Low |
| **Imports / modules** | Only `use std::host::stdio` | Low |

## Suggested New Benchmarks

| Proposed benchmark | Category | Primary features | Gap filled |
|--------------------|----------|-----------------|------------|
| `bench_closure_map.ark` | compute | Closures, higher-order functions, Vec | Closures |
| `bench_enum_dispatch.ark` | compute | Enum variants, match, function dispatch | Enum / pattern matching |
| `bench_struct_graph.ark` | memory | Nested structs, recursive types, allocation | Struct-heavy |
| `bench_error_chain.ark` | compute | Result, error propagation, match | Error paths |
| `bench_data_pipeline.ark` | application | Closures, Vec, struct iteration, filter/map/reduce | Multi-feature application pipeline |
| `bench_template_engine.ark` | application | HashMap, strings, recursion, template expansion | Template rendering |
| `bench_file_io.ark` | io | File read/write, String, I/O host calls | I/O-heavy |
| `bench_parser_lite.ark` | application | String, structs, enums, loops, recursion | Multi-feature application |

### Priority

1. **`bench_data_pipeline.ark`** — application-level closure pipeline with filter/map/aggregate stages.
2. **`bench_template_engine.ark`** — HashMap-backed template engine with recursive variable resolution.
3. **`bench_closure_map.ark`** — closures are a core language feature with dedicated coverage.
