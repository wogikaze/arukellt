# Component Model test fixtures & interop validation

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-03-30
**ID**: 033
**Depends on**: 030, 031
**Track**: component-model
**Blocks v1 exit**: no

## Summary

Create a comprehensive test suite for Component Model output. This includes
component-specific fixture tests (compile + validate), wasmtime component runner
integration, and cross-language interop tests demonstrating Arukellt components
interacting with components written in other languages.

## Context

The existing test infrastructure is fixture-driven via `tests/fixtures/manifest.txt`
with test kinds: `run:`, `diag:`, `module-run:`, `module-diag:`, `t3-compile:`.

Component Model testing requires new test kinds:
- `component-compile:` — compile to .component.wasm, validate binary
- `component-run:` — compile to component, execute via wasmtime component runner
- `component-interop:` — multi-component composition tests

## Acceptance Criteria

- [x] New test kind `component-compile:` added to `crates/arukellt/tests/harness.rs`.
      Compiles with `--emit component --target wasm32-wasi-p2`, validates output with
      `wasmparser` component validation.
- [x] New test kind `component-run:` added to harness. Compiles to component, runs via
      wasmtime with `--wasm component-model` flag, checks stdout.
- [x] At least 10 component fixture files created under `tests/fixtures/component/`:
      - `hello.ark` — minimal component with no exports (just _start)
      - `export_add.ark` — exports `pub fn add(a: i32, b: i32) -> i32`
      - `export_string.ark` — exports `pub fn greet(name: String) -> String`
      - `export_record.ark` — exports function taking/returning a struct
      - `export_enum.ark` — exports function with enum parameter
      - `export_result.ark` — exports function returning `Result<i32, String>`
      - `export_list.ark` — exports function taking `Vec<i32>`
      - `export_option.ark` — exports function with `Option<String>` param
      - `import_host.ark` + `host.wit` — imports a host function, calls it
      - `multi_export.ark` — multiple exported functions
- [x] All component fixtures are registered in `tests/fixtures/manifest.txt`.
- [x] At least 1 cross-language interop test:
      - An Arukellt component exporting `add(s32, s32) -> s32` composed with a Rust
        component that imports and calls it. Validated via `wasm-tools compose` or
        wasmtime component linking.
      - Test script in `tests/component-interop/` with build + run + validate steps.
- [x] WIT extraction test: for each component fixture, `wasm-tools component wit` on the
      output matches the expected WIT text (snapshot test).
- [x] `scripts/verify-harness.sh` extended with a component test gate (check 17).
- [x] Fixture count in `docs/current-state.md` updated.

## Key Files

- `crates/arukellt/tests/harness.rs` — new test kinds
- `tests/fixtures/component/` — new fixture directory
- `tests/fixtures/manifest.txt` — new entries
- `tests/component-interop/` — cross-language test scripts
- `scripts/verify-harness.sh` — component gate addition

## Notes

- `component-run:` tests require wasmtime with component model support. If wasmtime is
  not available in the test environment, these tests should be skipped with a clear message
  (not failed).
- Cross-language interop tests require Rust toolchain + `cargo-component`. These are
  opt-in (environment variable `ARUKELLT_TEST_INTEROP=1`) to avoid CI dependency bloat.
- Snapshot testing for WIT output: store expected `.wit` files alongside fixtures,
  compare with `wasm-tools component wit <output.wasm>`.
