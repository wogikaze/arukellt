# Changelog

All notable changes to the Arukellt project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [v5] — Unreleased (Self-Hosting)

### Added

- Self-hosted compiler in `src/compiler/*.ark`
- Bootstrap verification script (`scripts/verify-bootstrap.sh`)
- Frozen language specification (`docs/language/spec.md`)

### Changed

- Compiler can now be built from Arukellt source (Stage 0 → Stage 1 → Stage 2)

### Migration

- See `docs/migration/v4-to-v5.md`

---

## [v4] — Unreleased (Optimization)

### Added

- `--opt-level 0|1|2` CLI flag for optimization level control
- `--time` CLI flag for per-phase compile-time reporting
- Seven MIR optimization passes: `const_folding`, `dce`, `copy_propagation`, `inline`, `licm`, `escape_analysis`, `gc_hint`
- `ARUKELLT_DUMP_PHASES=optimized-mir` support
- Benchmark suite in `benchmarks/` (`binary_tree`, `vec_push_pop`, `string_concat`)
- Performance baselines in `tests/baselines/perf/`
- Backend peephole optimizations (T3): redundant local elimination, string dedup, dead branch removal

### Changed

- Perf gate thresholds added to `scripts/verify-harness.sh`

### Migration

- See `docs/migration/v3-to-v4.md`

---

## [v3] — Standard Library

### Added

- Module system: `use std::*` import paths
- Stdlib modules: `std::core`, `std::text`, `std::bytes`, `std::collections`, `std::fs`, `std::io`, `std::time`, `std::random`, `std::process`, `std::env`, `std::cli`, `std::wasm`, `std::wit`, `std::component`
- Scalar type completeness across all stdlib modules
- Stability labels (Stable / Unstable / Deprecated) for public API
- Generated stdlib reference documentation (`docs/stdlib/reference.md`)
- Prelude migration guide (`docs/stdlib/prelude-migration.md`)
- Stdlib fixture integration in `verify-harness.sh`

### Changed

- Monomorphic helper functions reorganized into named modules
- Prelude scope narrowed; some functions require explicit `use` imports

### Migration

- See `docs/migration/v2-to-v3.md`

---

## [v2] — Component Model

### Added

- `--emit component` for Component Model binary output (`.component.wasm`)
- `--emit wit` for WIT interface generation
- `--emit all` for combined core + component output
- `--wit <path>` for host import WIT binding
- `W0005` diagnostic for non-exportable `pub fn`
- WIT type mapping: `i32`→`s32`, `i64`→`s64`, `f64`→`f64`, `bool`→`bool`, `String`→`string`, `Vec<T>`→`list<T>`, `Option<T>`→`option<T>`, `Result<T,E>`→`result<T,E>`

### Known Limitations

- Requires external `wasm-tools` and WASI adapter module
- Async Component Model features not supported
- Some canonical ABI lift/lower paths incomplete

### Migration

- See `docs/migration/v1-to-v2.md`

---

## [v1] — GC-Native Core

### Added

- T3 (`wasm32-wasi-p2`) GC-native backend: all values in Wasm GC heap
- Trait definitions and `impl` blocks with static dispatch
- Operator overloading (`+`, `-`, `*`, `/`, `==`, `!=`, `<`, `<=`, `>`, `>=`)
- Pattern matching extensions: guards, or-patterns, struct/tuple patterns
- Struct field update syntax (`{ field: val, ..base }`)
- Nested generics and user-defined generic structs
- `W0004` diagnostic for backend validation failure
- T1 (`wasm32-wasi-p1`) retained as compatibility path

### Breaking Changes

- `parse_i64` / `parse_f64` return `Result<T, String>` (was raw value)
- `trait`, `impl`, `for`, `in` are reserved keywords

### Migration

- See `docs/migration/v0-to-v1.md`
