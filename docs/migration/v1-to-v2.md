# Migration Guide: v1 → v2 (Component Model)

> Updated: 2026-03-28
> **Current-first note**: this guide explains the v1→v2 component-model transition. For the current support matrix and known limitations, also check [`../current-state.md`](../current-state.md).

## Overview

v2 adds Component Model support on top of the v1 T1/T3 core Wasm paths. Existing v1 code continues to compile and run as core Wasm, while `wasm32-wasi-p2` gains `--emit component`, `--emit wit`, and `--emit all` for the currently supported export surface.

## New CLI Flags

### `--emit component`

Compile an Arukellt source file to a Component Model binary (`.component.wasm`):

```bash
arukellt compile --emit component mylib.ark --target wasm32-wasi-p2
```

This produces `mylib.component.wasm` for the supported component export surface.

**Requirements:**
- `wasm-tools` must be installed: `cargo install wasm-tools`
- A WASI adapter module must be available via `ARK_WASI_ADAPTER=/path/to/adapter.wasm` or the expected local path

### `--wit <path>`

Specify WIT files for host import binding:

```bash
arukellt compile --emit component myapp.ark --wit host.wit --target wasm32-wasi-p2
```

Multiple `--wit` flags are supported.

### `--emit wit`

Generate a WIT description of the module's export surface:

```bash
arukellt compile --emit wit mylib.ark --target wasm32-wasi-p2
```

### `--emit all`

Produce both core Wasm and component binary:

```bash
arukellt compile --emit all mylib.ark --target wasm32-wasi-p2
```

## Export Surface

`pub fn` declarations with currently supported WIT-compatible parameter and return types are exported at the component boundary.

```arukellt
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn helper() -> i32 { 42 }

pub fn apply(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}
```

In this example, `add` is exportable, `helper` is not exported because it is not `pub`, and `apply` is not exportable because closure/function types are not part of the currently supported WIT export surface.

### WIT-compatible types (documented mapping target)

| Arukellt type | WIT type |
|--------------|----------|
| `i32`        | `s32` |
| `i64`        | `s64` |
| `f32`        | `f32` |
| `f64`        | `f64` |
| `bool`       | `bool` |
| `String`     | `string` |
| `Vec<T>`     | `list<T>` |
| `Option<T>`  | `option<T>` |
| `Result<T,E>`| `result<T,E>` |

See `docs/current-state.md` for the current note about which canonical ABI cases are fully wired in today.

### Naming convention

Function names are converted to kebab-case in generated WIT (for example `is_even` → `is-even`).

## Known Limitations

- Component wrapping depends on external `wasm-tools`.
- A WASI adapter module is still required for wrapping current core Wasm outputs.
- Not every string / list / complex canonical ABI lift/lower path is fully wired into the emitter yet.
- Async Component Model features (streams, futures) are not supported.
- Cross-interface `use` declarations are not fully supported.
- Resource handle support is planned / partially represented, but full lifecycle behavior is not the universal default surface yet.

## Diagnostic Changes

### `W0005` — Non-exportable function

```text
warning[W0005]: function `apply` has non-exportable parameter type(s): Closure
```

This warning is emitted when a `pub fn` cannot be represented on the current component export surface.

## Unchanged Behavior

- T1 (`wasm32-wasi-p1`) core Wasm compilation remains available.
- T3 (`wasm32-wasi-p2`) core Wasm compilation remains available.
- `run` and `check` continue to work as before for the core Wasm path.
- Existing core-Wasm-oriented workflows do not need to adopt component output.
