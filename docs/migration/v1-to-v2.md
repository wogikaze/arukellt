# Migration Guide: v1 → v2 (Component Model)

> Updated: 2026-03-28

## Overview

v2 adds Component Model support to Arukellt. All v1 functionality continues to work
without modification. v2 is an additive release — there are no breaking changes to
existing v1 code.

## New CLI Flags

### `--emit component`

Compile an Arukellt source file to a Component Model binary (`.component.wasm`):

```bash
arukellt compile --emit component mylib.ark --target wasm32-wasi-p2
```

This produces `mylib.component.wasm` which can be consumed by any Component Model
runtime (e.g., wasmtime, jco).

**Requirements:**
- `wasm-tools` must be installed: `cargo install wasm-tools`
- A WASI adapter module (`wasi_snapshot_preview1.reactor.wasm`) must be available.
  Set `ARK_WASI_ADAPTER=/path/to/adapter.wasm` or place it in the current directory.

### `--wit <path>`

Specify WIT files for host import binding:

```bash
arukellt compile --emit component myapp.ark --wit host.wit --target wasm32-wasi-p2
```

Multiple `--wit` flags are supported for multiple interface files.

### `--emit wit`

Generate a WIT description of the module's export surface:

```bash
arukellt compile --emit wit mylib.ark --target wasm32-wasi-p2
```

This produces `mylib.wit` describing all `pub fn` with WIT-compatible signatures.

### `--emit all`

Produce both core Wasm and component binary:

```bash
arukellt compile --emit all mylib.ark --target wasm32-wasi-p2
```

## Export Surface

In v2, `pub fn` declarations with WIT-compatible parameter and return types are
automatically exported at the component boundary:

```arukellt
// Exported as component function
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// NOT exported (not pub)
fn helper() -> i32 { 42 }

// NOT exported (closure parameter not WIT-compatible, W0005 warning)
pub fn apply(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}
```

### WIT-compatible types

| Arukellt type | WIT type |
|--------------|----------|
| `i32`        | `s32`    |
| `i64`        | `s64`    |
| `f32`        | `f32`    |
| `f64`        | `f64`    |
| `bool`       | `bool`   |
| `String`     | `string` |
| `Vec<T>`     | `list<T>` |
| `Option<T>`  | `option<T>` |
| `Result<T,E>`| `result<T,E>` |

### Naming convention

Function names are converted to kebab-case in WIT (e.g., `is_even` → `is-even`).
The Component Model canonical ABI handles this mapping automatically.

## Known Limitations

- **External tooling dependency**: Component wrapping requires `wasm-tools` as an
  external subprocess. This will be replaced by an in-tree implementation in v3.
- **WASI adapter required**: Core modules with WASI imports need a WASI adapter
  module for component wrapping.
- **Scalar exports only**: String/list/complex type passing at component boundaries
  requires canonical ABI lift/lower helpers (implemented but not yet wired into
  the emitter for all types).
- **No async support**: Async component model features (streams, futures) are
  deferred to v5.
- **No WIT `use`**: Cross-interface `use` declarations are not yet supported.
- **No resource lifecycle**: `own<T>` and `borrow<T>` handle passing is parsed
  and planned but not yet emitted in the Wasm backend.

## Diagnostic Changes

### New: W0005 — Non-exportable function

```
warning[W0005]: function `apply` has non-exportable parameter type(s): Closure
```

Emitted when a `pub fn` has parameter or return types that cannot be represented
in the Component Model WIT export surface.

## Unchanged Behavior

- T1 (wasm32-wasi-p1) compilation is unchanged.
- T3 (wasm32-wasi-p2) core Wasm compilation is unchanged.
- All existing fixtures continue to pass.
- The `run` subcommand is unchanged.
- The `check` subcommand is unchanged.
