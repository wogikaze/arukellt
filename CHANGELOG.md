# Changelog

All notable changes to the Arukellt project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [v5] — Self-Hosting

### Highlights

- **Selfhost fixpoint reached**: `sha256(s2) == sha256(s3)` passes — the selfhost compiler
  reproducibly compiles itself. All four selfhost gates (fixpoint, fixture-parity, diag-parity,
  CLI parity) exit 0 with 0 failures.
- **Selfhost-only execution path**: the `arukellt` CLI now runs exclusively through the selfhost
  wasm under `wasmtime`. The legacy Rust compilation path is fully retired.
- **Phase 5–7 crate removals**: `crates/ark-driver`, `crates/ark-mir`, legacy Rust Wasm
  emitter, `crates/ark-llvm` scaffold, and `crates/ark-lsp` have all been removed.
- **Stdlib modernization**: sentinel-value APIs replaced with typed `Option`/`Result`/enum
  surfaces; `std::host::fs` and `std::json` error types changed to typed enums (`FsError`,
  `JsonParseError`).
- **Phase 6 IDE**: LSP error recovery, partial AST, incremental diagnostics, hover, and
  definition handlers are now implemented in `src/ide/lsp.ark`.

### Added

- Self-hosted compiler sources in `src/compiler/*.ark`; bootstrap pinned reference at
  `bootstrap/arukellt-selfhost.wasm` (ADR-029, #585)
- `scripts/run/arukellt-selfhost.sh` — thin wrapper that locates and runs the selfhost
  wasm under `wasmtime` (#559, #583)
- Selfhost gates in `python scripts/manager.py selfhost`:
  - `selfhost fixpoint` — Stage-3 sha256 fixpoint check
  - `selfhost fixture-parity` — 321 PASS / 0 FAIL pinned-vs-current parity
  - `selfhost diag-parity` — 12 PASS / 0 FAIL diagnostic golden parity
  - `selfhost parity --cli` — 6 PASS / 0 FAIL CLI snapshot parity (#556–#558)
- Phase 6 IDE / LSP sources in `src/ide/`:
  - Partial AST recovery on parse errors (#566)
  - Incremental diagnostic accumulation (#567)
  - Analysis API (`text → AST → symbols → diagnostics`) (#568)
  - `initialize` / `didOpen` / `didChange` / `publishDiagnostics` handlers (#569)
  - Hover and go-to-definition handlers (#570)
- `std::host::fs` typed error enum `FsError` (#515, #525):
  - `FsError::NotFound(String)`
  - `FsError::PermissionDenied(String)`
  - `FsError::Utf8Error`
  - `FsError::IoError(String)`
  - Helper: `fs_error_message(err: FsError) -> String`
- `std::json` typed parse error enum `JsonParseError` (#521):
  - Variants: `EmptyInput`, `InvalidLiteral`, `TrailingCharacters`, `UnexpectedCharacter`
- Benchmark real-world workload suite: `http_parser`, `log_processor`, `config_loader` (#545)
- New cpu/memory/compute benchmarks: `closure_map`, `enum_dispatch`, `struct_graph`,
  `error_chain` (#539–#542) following `bench_<suite>_<name>.ark` naming convention (#544)
- `docs/stdlib/604-contract-honesty-gap-ledger.md` — honesty caveats for stdlib APIs (#604)
- Verification unified under `python scripts/manager.py`; shell script wrappers retired
  (#532–#537)

### Changed

- `arukellt` CLI execution path: selfhost wasm under `wasmtime` is now the **sole** path.
  Resolution order: `$ARUKELLT_SELFHOST_WASM` → `.build/selfhost/arukellt-s2.wasm` →
  `.bootstrap-build/arukellt-s2.wasm` → `bootstrap/arukellt-selfhost.wasm` (#559, #583)
- `std::host::fs::read_to_string(path) -> Result<String, FsError>`
  (was `Result<String, String>`) (#515)
- `std::host::fs::write_string(path, contents) -> Result<(), FsError>`
  (was `Result<(), String>`) (#515)
- `std::host::fs::write_bytes(path, bytes) -> Result<(), FsError>`
  (was `Result<(), String>`) (#515)
- `std::json::parse(s) -> Result<JsonValue, JsonParseError>`
  (was `Result<JsonValue, String>`) (#521)
- `std::fs::exists(path)` documented as a **readable-file probe** — returns `false`
  for directories and unreadable paths (#524)
- Stdlib enum/match syntax modernized throughout (#511)
- Prelude scope narrowed: some functions now require explicit `use` imports (#513)
- All verification via `python scripts/manager.py` domain subcommands:
  `verify`, `selfhost`, `docs`, `perf`, `gate` (#532–#536)
- Benchmark suite reorganized under `bench_<suite>_<name>.ark` convention; legacy fixtures
  (`fib`, `binary_tree`, `vec_ops`, `string_concat`) moved to `benchmarks/legacy/` (#544)

### Removed (Breaking)

- **`ARUKELLT_USE_RUST=1`**: exits non-zero with a migration notice; Rust compilation
  path no longer exists (#583)
- **`cargo run -p arukellt`** no longer used in `scripts/` or `.github/workflows/` (#559)
- `crates/ark-driver` — removed (#560)
- `crates/ark-mir` — removed (#561); selfhost `src/compiler/passes/` is the source of
  truth for MIR optimization passes
- Rust Wasm emitter crate — removed (#562)
- `crates/ark-llvm` — LLVM scaffold removed (#586)
- `crates/ark-lsp` — removed (#572); LSP server now runs from `src/ide/lsp.ark`
- Legacy shell scripts replaced by `python scripts/manager.py` equivalents (#537)

### Known Limitations

- WASI Preview 2 native components (without P1 adapter) deferred to v5+ (#074)
- Component output is T3-only (`--target wasm32-wasi-p2`)
- Tier 2/3 export types produce compile errors (canonical ABI lift/lower incomplete)
- `std::host::sockets` is T3-only; use on T1 produces E0500
- `std::host::http` uses TCP HTTP/1.1; HTTPS not supported
- Dead function elimination disabled for T3
- `std::fs::exists(path)` is a readable-file probe, not a general path-existence check (#605)

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

- Perf gate thresholds added to `scripts/manager.py`

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
