# selfhost: connect CLI main.ark to driver pipeline

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 209
**Depends on**: 208
**Track**: compiler/selfhost
**Blocks v1 exit**: no

**Status note**: `src/compiler/main.ark` is wired (agent already added `use driver` and the dispatch block) but cannot be built until #208 is resolved — the module private-items bug prevents `driver.ark` from compiling.

## Summary

`src/compiler/main.ark:196` is the CLI entry. It parses args, builds a `DriverConfig`, and must call `driver::compile_file(input, config)`. The `src/compiler/driver.ark` pipeline is already fully qualified (all module calls use `lexer::`, `parser::`, etc.) but the containing file can't be resolved until #208 is fixed.

Output handling:
- If `-o <file>` is given: call `fs::write_bytes(output_file, result.output_bytes)` (already added to `std/host/fs.ark` and backed by `__intrinsic_fs_write_bytes`)
- Otherwise: print byte count to stdout
- On error: print error messages to stderr, exit 1

## Acceptance

- [ ] `arukellt compile src/compiler/main.ark -o /tmp/compiler.wasm` succeeds (no build errors)
- [ ] `wasmtime --dir=. /tmp/compiler.wasm -- compile tests/fixtures/stdlib_cli/cli_basic.ark` runs (even if compilation is partial/stub)
- [ ] `wasmtime --dir=. /tmp/compiler.wasm -- --help` prints usage
- [ ] All existing harness tests still pass

## References

- `src/compiler/main.ark`
- `src/compiler/driver.ark`
- `std/host/fs.ark` (`write_bytes`)
- `crates/ark-typecheck/src/checker/builtins.rs` (`__intrinsic_fs_write_bytes` signature)
- `crates/ark-wasm/src/emit/t1/operands.rs` (T1 implementation)
